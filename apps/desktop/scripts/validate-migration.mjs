import { DatabaseSync } from 'node:sqlite';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const migrationPath = resolve(scriptDir, '../src-tauri/migrations/0001_initial.sql');

let migrationSql;
try {
  migrationSql = readFileSync(migrationPath, 'utf8');
} catch {
  console.error('MIGRATION VALIDATE FAIL');
  console.error('reason: could not read migration file');
  process.exit(1);
}

const db = new DatabaseSync(':memory:');

// foreign_keys must be enabled
db.exec('PRAGMA foreign_keys = ON;');

// Run the migration
try {
  db.exec(migrationSql);
} catch (error) {
  console.error('MIGRATION VALIDATE FAIL');
  console.error('reason: migration did not execute');
  console.error(`detail: ${error?.message ?? 'unknown'}`);
  process.exit(1);
}

let testCount = 0;
let passCount = 0;

function test(name, fn) {
  testCount++;
  try {
    fn();
    passCount++;
    console.log(`PASS [${testCount}] ${name}`);
  } catch (e) {
    console.error(`FAIL [${testCount}] ${name}`);
    console.error(`      ${e.message}`);
    process.exitCode = 1;
  }
}

// ── Helper: execute a prepared statement, return true on success, false on error ──

function tryExec(sql, ...params) {
  try {
    db.prepare(sql).run(...params);
    return true;
  } catch {
    return false;
  }
}

// ── Verify required tables ──

test('all required tables exist', () => {
  const requiredTables = [
    'provider_profiles', 'books', 'chapters', 'scan_jobs',
    'rule_selections', 'checkpoints', 'findings', 'evidence',
  ];
  for (const table of requiredTables) {
    const result = db.prepare(
      "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?",
    ).get(table);
    if (!result) throw new Error(`required table not found: ${table}`);
  }
});

// ── foreign_keys is ON ──

test('PRAGMA foreign_keys is ON', () => {
  const fkResult = db.prepare('PRAGMA foreign_keys').get();
  if (!fkResult || fkResult.foreign_keys !== 1) {
    throw new Error('PRAGMA foreign_keys is not ON');
  }
});

// ═══════════════════════════════════════════════════════════
// Item 六: credential_ref hardening tests
// ═══════════════════════════════════════════════════════════

test('credential_ref: valid secret-ref insert succeeds', () => {
  const ok = tryExec(
    `INSERT INTO provider_profiles (id, provider_kind, display_name, base_url, credential_ref, credential_state, created_at_ms, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    'test-provider', 'openai', 'Test Provider', 'https://example.com', 'secret-ref:test-profile-id', 'configured', 0, 0
  );
  if (!ok) throw new Error('valid secret-ref insert failed');
  db.exec("DELETE FROM provider_profiles WHERE id = 'test-provider'");
});

test('credential_ref: NULL credential_ref succeeds', () => {
  const ok = tryExec(
    `INSERT INTO provider_profiles (id, provider_kind, display_name, base_url, credential_ref, credential_state, created_at_ms, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    'null-cred', 'openai', 'Null Cred', 'https://example.com', null, 'missing', 0, 0
  );
  if (!ok) throw new Error('NULL credential_ref insert failed');
  db.exec("DELETE FROM provider_profiles WHERE id = 'null-cred'");
});

test('credential_ref: invalid credential_state rejected', () => {
  if (tryExec(
    `INSERT INTO provider_profiles (id, provider_kind, display_name, base_url, credential_ref, credential_state, created_at_ms, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    'bad-state', 'openai', 'Bad State', 'https://example.com', null, 'not_configured', 0, 0
  )) {
    throw new Error('invalid credential_state "not_configured" was not rejected');
  }
});

// Negative credential_ref tests — verify ck_provider_credential_ref constraint name

function assertCredentialRefRejected(value, label) {
  // credential_state='missing' — valid so only credential_ref CHECK is tested
  const testId = 'bad-cred-' + testCount;
  const sql = `INSERT INTO provider_profiles (id, provider_kind, display_name, base_url, credential_ref, credential_state, created_at_ms, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`;
  try {
    db.prepare(sql).run(testId, 'openai', 'Bad', 'https://example.com', value, 'missing', 0, 0);
    throw new Error(`credential_ref not rejected (${label})`);
  } catch (e) {
    if (!e.message.includes('ck_provider_credential_ref')) {
      throw new Error(`wrong constraint failed for (${label}): ${e.message}`);
    }
  }
}

test('credential_ref: plaintext API key "sk-test" rejected', () => {
  assertCredentialRefRejected('sk-test', 'plaintext API key "sk-test"');
});

test('credential_ref: plaintext "api-key-123" rejected', () => {
  assertCredentialRefRejected('api-key-123', 'plaintext "api-key-123"');
});

test('credential_ref: plaintext "mytoken" rejected', () => {
  assertCredentialRefRejected('mytoken', 'plaintext "mytoken"');
});

test('credential_ref: plaintext "Bearer xxx" rejected', () => {
  assertCredentialRefRejected('Bearer xxx', 'plaintext "Bearer xxx"');
});

test('credential_ref: uppercase prefix "SECRET-REF:x" rejected', () => {
  assertCredentialRefRejected('SECRET-REF:x', 'uppercase prefix "SECRET-REF:x"');
});

test('credential_ref: empty suffix "secret-ref:" rejected', () => {
  assertCredentialRefRejected('secret-ref:', 'empty suffix "secret-ref:"');
});

test('credential_ref: whitespace-only suffix rejected', () => {
  assertCredentialRefRejected('secret-ref:   ', 'whitespace-only suffix');
});

test('credential_ref: leading whitespace "  secret-ref:x" rejected', () => {
  assertCredentialRefRejected('  secret-ref:x', 'leading whitespace "  secret-ref:x"');
});

test('credential_ref: trailing whitespace "secret-ref:x  " rejected', () => {
  assertCredentialRefRejected('secret-ref:x  ', 'trailing whitespace "secret-ref:x  "');
});

test('credential_ref: newline in suffix "secret-ref:x\\n" rejected', () => {
  assertCredentialRefRejected('secret-ref:x\n', 'newline in suffix');
});

test('credential_ref: tab in suffix "secret-ref:x\\ty" rejected', () => {
  assertCredentialRefRejected('secret-ref:x\ty', 'tab in suffix');
});

test('credential_ref: invalid suffix chars "secret-ref:!@#$" rejected', () => {
  assertCredentialRefRejected('secret-ref:!@#$', 'invalid suffix characters "!@#$"');
});

test('credential_ref: valid "secret-ref:x" accepted', () => {
  const ok = tryExec(
    `INSERT INTO provider_profiles (id, provider_kind, display_name, base_url, credential_ref, credential_state, created_at_ms, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    'cred-ok-x', 'openai', 'Cred OK x', 'https://example.com', 'secret-ref:x', 'configured', 0, 0
  );
  if (!ok) throw new Error('valid secret-ref:x insert failed');
  db.exec("DELETE FROM provider_profiles WHERE id = 'cred-ok-x'");
});

test('credential_ref: valid "secret-ref:A0._-" accepted', () => {
  const ok = tryExec(
    `INSERT INTO provider_profiles (id, provider_kind, display_name, base_url, credential_ref, credential_state, created_at_ms, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    'cred-ok-alnum', 'openai', 'Cred OK alnum', 'https://example.com', 'secret-ref:A0._-', 'configured', 0, 0
  );
  if (!ok) throw new Error('valid secret-ref:A0._- insert failed');
  db.exec("DELETE FROM provider_profiles WHERE id = 'cred-ok-alnum'");
});

test('credential_ref: valid "secret-ref:valid-id" accepted', () => {
  const ok = tryExec(
    `INSERT INTO provider_profiles (id, provider_kind, display_name, base_url, credential_ref, credential_state, created_at_ms, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    'cred-ok-valid', 'openai', 'Cred OK valid', 'https://example.com', 'secret-ref:valid-id', 'configured', 0, 0
  );
  if (!ok) throw new Error('valid secret-ref:valid-id insert failed');
  db.exec("DELETE FROM provider_profiles WHERE id = 'cred-ok-valid'");
});

test('credential_ref: valid 256-char total length accepted', () => {
  const suffix = 'a'.repeat(245); // 'secret-ref:' = 11 chars, total 11 + 245 = 256
  const value = 'secret-ref:' + suffix;
  if (value.length !== 256) throw new Error(`expected 256, got ${value.length}`);
  const ok = tryExec(
    `INSERT INTO provider_profiles (id, provider_kind, display_name, base_url, credential_ref, credential_state, created_at_ms, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    'cred-ok-256', 'openai', 'Cred OK 256', 'https://example.com', value, 'configured', 0, 0
  );
  if (!ok) throw new Error('valid 256-char secret-ref insert failed');
  db.exec("DELETE FROM provider_profiles WHERE id = 'cred-ok-256'");
});

test('credential_ref: empty suffix "secret-ref:" rejected', () => {
  assertCredentialRefRejected('secret-ref:', 'empty suffix');
});

test('credential_ref: "secret-ref:ab!@#$" rejected (special chars after valid prefix)', () => {
  assertCredentialRefRejected('secret-ref:ab!@#$', 'ab!@#$');
});

test('credential_ref: "secret-ref:ab/path" rejected (slash in suffix)', () => {
  assertCredentialRefRejected('secret-ref:ab/path', 'ab/path');
});

test('credential_ref: "secret-ref:ab\\\\path" rejected (backslash in suffix)', () => {
  assertCredentialRefRejected('secret-ref:ab\\path', 'ab\\path');
});

test('credential_ref: "secret-ref:ab cd" rejected (space in suffix)', () => {
  assertCredentialRefRejected('secret-ref:ab cd', 'ab cd');
});

test('credential_ref: "secret-ref:ab\\u000b" rejected (vertical tab in suffix)', () => {
  assertCredentialRefRejected('secret-ref:ab', 'ab\\u000b');
});

test('credential_ref: "secret-ref:ab\\u00a0" rejected (non-breaking space in suffix)', () => {
  assertCredentialRefRejected('secret-ref:ab ', 'ab\\u00a0');
});

test('credential_ref: "secret-ref:ab\\u4e2d" rejected (CJK character in suffix)', () => {
  assertCredentialRefRejected('secret-ref:ab中', 'ab中');
});

test('credential_ref: "secret-ref:ab\\\\n" rejected (newline after valid chars)', () => {
  assertCredentialRefRejected('secret-ref:ab\n', 'ab\\n');
});

test('credential_ref: "secret-ref:ab\\\\t" rejected (tab after valid chars)', () => {
  assertCredentialRefRejected('secret-ref:ab\t', 'ab\\t');
});

test('credential_ref: 257-char total length rejected', () => {
  const suffix = 'a'.repeat(246); // 'secret-ref:' = 11 chars, total 11 + 246 = 257
  const value = 'secret-ref:' + suffix;
  if (value.length !== 257) throw new Error(`expected 257, got ${value.length}`);
  assertCredentialRefRejected(value, '257-char');
});

// ═══════════════════════════════════════════════════════════
// Item 四: Setup test data with rule_pack snapshots
// ═══════════════════════════════════════════════════════════

test('scan_job INSERT fails without rule_pack_id_snapshot', () => {
  if (tryExec(
    `INSERT INTO books (id, title, source_display_name, source_format, fingerprint, imported_at_ms, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?)`,
    'tmp-book-rp', 'Tmp', 'tmp.txt', 'plain_text', 'abcd0000000001', 0, 0
  )) {
    const ok = tryExec(
      `INSERT INTO scan_jobs (id, book_id, status, provider_kind_snapshot, model_id_snapshot, rule_pack_version_snapshot, created_at_ms, updated_at_ms)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
      'no-rp-id', 'tmp-book-rp', 'pending', 'openai', 'gpt-test', '1.0', 0, 0
    );
    db.exec("DELETE FROM scan_jobs WHERE id = 'no-rp-id'");
    db.exec("DELETE FROM books WHERE id = 'tmp-book-rp'");
    if (ok) throw new Error('scan_job INSERT without rule_pack_id_snapshot should have been rejected');
  } else {
    db.exec("DELETE FROM books WHERE id = 'tmp-book-rp'");
  }
});

// Setup test books
db.exec(`
  INSERT INTO books (id, title, source_display_name, source_format, fingerprint, imported_at_ms, updated_at_ms)
  VALUES ('test-book', 'Test', 'test-book.txt', 'plain_text', 'deadbeef00000001', 0, 0)
`);
db.exec(`
  INSERT INTO books (id, title, source_display_name, source_format, fingerprint, imported_at_ms, updated_at_ms)
  VALUES ('test-book-2', 'Test 2', 'test-book-2.txt', 'plain_text', 'deadbeef00000002', 0, 0)
`);

// Setup test chapters
db.exec(`
  INSERT INTO chapters (id, book_id, ordinal, title, body, content_hash, source_locator_json, character_count, created_at_ms)
  VALUES ('test-ch', 'test-book', 0, 'Ch1', 'body text', 'deadbeef00000003', '{}', 9, 0)
`);
db.exec(`
  INSERT INTO chapters (id, book_id, ordinal, title, body, content_hash, source_locator_json, character_count, created_at_ms)
  VALUES ('test-ch-2', 'test-book-2', 0, 'Ch2', 'body text 2', 'deadbeef00000004', '{}', 11, 0)
`);

// Setup test scan_jobs WITH rule_pack snapshots
test('scan_job INSERT with rule_pack snapshots succeeds', () => {
  if (!tryExec(
    `INSERT INTO scan_jobs (id, book_id, status, provider_kind_snapshot, model_id_snapshot, rule_pack_id_snapshot, rule_pack_version_snapshot, created_at_ms, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    'test-job', 'test-book', 'pending', 'openai', 'gpt-test', 'test-pack', '1.0', 0, 0
  )) {
    throw new Error('scan_job INSERT with rule_pack snapshots failed');
  }
});

db.exec(`
  INSERT INTO scan_jobs (id, book_id, status, provider_kind_snapshot, model_id_snapshot, rule_pack_id_snapshot, rule_pack_version_snapshot, created_at_ms, updated_at_ms)
  VALUES ('test-job-2', 'test-book-2', 'pending', 'openai', 'gpt-test', 'test-pack', '1.0', 0, 0)
`);

// ═══════════════════════════════════════════════════════════
// Item 四: Checkpoint v2 round-trip tests
// ═══════════════════════════════════════════════════════════

test('checkpoint: schema_version=2 with scan_profile_fingerprint succeeds', () => {
  if (!tryExec(
    `INSERT INTO checkpoints (scan_job_id, schema_version, document_fingerprint, scan_profile_fingerprint, next_chapter_position, processed_chapters_json, context_snapshot_json, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    'test-job', 2, 'docfp-001', 'profilefp-001', 0, '[]', '{}', 1000
  )) {
    throw new Error('checkpoint v2 insert failed');
  }
});

test('checkpoint: read back verifies all fields', () => {
  const row = db.prepare('SELECT * FROM checkpoints WHERE scan_job_id = ?').get('test-job');
  if (!row) throw new Error('checkpoint not found');
  if (row.schema_version !== 2) throw new Error(`expected schema_version=2, got ${row.schema_version}`);
  if (row.document_fingerprint !== 'docfp-001') throw new Error('document_fingerprint mismatch');
  if (row.scan_profile_fingerprint !== 'profilefp-001') throw new Error('scan_profile_fingerprint mismatch');
  if (row.next_chapter_position !== 0) throw new Error('next_chapter_position mismatch');
});

test('checkpoint: schema_version=1 is REJECTED', () => {
  db.exec("DELETE FROM checkpoints WHERE scan_job_id = 'test-job-2'");
  if (tryExec(
    `INSERT INTO checkpoints (scan_job_id, schema_version, document_fingerprint, scan_profile_fingerprint, next_chapter_position, processed_chapters_json, context_snapshot_json, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    'test-job-2', 1, 'docfp-002', 'profilefp-002', 0, '[]', '{}', 1000
  )) {
    throw new Error('schema_version=1 should have been rejected');
  }
});

test('checkpoint: schema_version=3 is REJECTED', () => {
  if (tryExec(
    `INSERT INTO checkpoints (scan_job_id, schema_version, document_fingerprint, scan_profile_fingerprint, next_chapter_position, processed_chapters_json, context_snapshot_json, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    'test-job-2', 3, 'docfp-003', 'profilefp-003', 0, '[]', '{}', 1000
  )) {
    throw new Error('schema_version=3 should have been rejected');
  }
});

test('checkpoint: empty scan_profile_fingerprint is REJECTED', () => {
  if (tryExec(
    `INSERT INTO checkpoints (scan_job_id, schema_version, document_fingerprint, scan_profile_fingerprint, next_chapter_position, processed_chapters_json, context_snapshot_json, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    'test-job-2', 2, 'docfp-004', '', 0, '[]', '{}', 1000
  )) {
    throw new Error('empty scan_profile_fingerprint should have been rejected');
  }
});

// ═══════════════════════════════════════════════════════════
// Item 四: scan_jobs immutability after checkpoint trigger
// ═══════════════════════════════════════════════════════════

test('scan_jobs: cannot modify provider_kind_snapshot after checkpoint', () => {
  try {
    db.exec("UPDATE scan_jobs SET provider_kind_snapshot = 'changed' WHERE id = 'test-job'");
    throw new Error('should have been rejected');
  } catch (e) {
    if (!e.message.includes('cannot modify scan_job fields after checkpoint exists')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

test('scan_jobs: can modify current_chapter_position after checkpoint', () => {
  // This field is NOT in the immutable list, so it should be updatable
  try {
    db.exec("UPDATE scan_jobs SET total_chapters = 10, current_chapter_position = 5 WHERE id = 'test-job'");
  } catch (e) {
    throw new Error(`should have succeeded but got: ${e.message}`);
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Setup rule_selections for trigger tests
// ═══════════════════════════════════════════════════════════

// Clean checkpoint from test-job so we can use test-job for finding tests without immutability interference on non-snapshot fields
// (checkpoint on test-job remains; it only locks the 6 snapshot/config columns)

// Insert rule_selections for FK and trigger tests
test('rule_selections: insert rule-fk succeeds', () => {
  if (!tryExec(
    `INSERT INTO rule_selections (scan_job_id, rule_id, rule_version, effective_category, alert_level, enabled)
     VALUES (?, ?, ?, ?, ?, ?)`,
    'test-job', 'rule-fk', 1, 'landmine', 'medium', 1
  )) {
    throw new Error('could not insert rule_selections rule-fk');
  }
});

test('rule_selections: insert rule-crossbook succeeds', () => {
  if (!tryExec(
    `INSERT INTO rule_selections (scan_job_id, rule_id, rule_version, effective_category, alert_level, enabled)
     VALUES (?, ?, ?, ?, ?, ?)`,
    'test-job', 'rule-crossbook', 1, 'landmine', 'medium', 1
  )) {
    throw new Error('could not insert rule_selections rule-crossbook');
  }
});

test('rule_selections: insert rule-cat-mismatch (frustration) succeeds', () => {
  if (!tryExec(
    `INSERT INTO rule_selections (scan_job_id, rule_id, rule_version, effective_category, alert_level, enabled)
     VALUES (?, ?, ?, ?, ?, ?)`,
    'test-job', 'rule-cat-mismatch', 1, 'frustration', 'medium', 1
  )) {
    throw new Error('could not insert rule-cat-mismatch');
  }
});

test('rule_selections: insert rule-al-mismatch (high) succeeds', () => {
  if (!tryExec(
    `INSERT INTO rule_selections (scan_job_id, rule_id, rule_version, effective_category, alert_level, enabled)
     VALUES (?, ?, ?, ?, ?, ?)`,
    'test-job', 'rule-al-mismatch', 1, 'landmine', 'high', 1
  )) {
    throw new Error('could not insert rule-al-mismatch');
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Illegal effective_category rejected (rule_selections CHECK)
// ═══════════════════════════════════════════════════════════

const badCategories = ['thunder', 'discomfort', 'unknown', '', 'CRITICAL', 'Landmine'];
for (const cat of badCategories) {
  test(`rule_selections: illegal effective_category rejected: "${cat}"`, () => {
    db.prepare("DELETE FROM rule_selections WHERE rule_id = 'rule-cat-test'").run();
    if (tryExec(
      `INSERT INTO rule_selections (scan_job_id, rule_id, rule_version, effective_category, alert_level, enabled)
       VALUES (?, ?, ?, ?, ?, ?)`,
      'test-job', 'rule-cat-test', 1, cat, 'medium', 1
    )) {
      throw new Error(`illegal effective_category not rejected: "${cat}"`);
    }
  });
}

// ═══════════════════════════════════════════════════════════
// Item 五: Illegal alert_level rejected (rule_selections CHECK)
// ═══════════════════════════════════════════════════════════

const badLevels = ['CRITICAL', 'severe', 'warning', '', '1', '5'];
for (const level of badLevels) {
  test(`rule_selections: illegal alert_level rejected: "${level}"`, () => {
    db.prepare("DELETE FROM rule_selections WHERE rule_id = 'rule-al-test'").run();
    if (tryExec(
      `INSERT INTO rule_selections (scan_job_id, rule_id, rule_version, effective_category, alert_level, enabled)
       VALUES (?, ?, ?, ?, ?, ?)`,
      'test-job', 'rule-al-test', 1, 'landmine', level, 1
    )) {
      throw new Error(`illegal alert_level not rejected: "${level}"`);
    }
  });
}

// ═══════════════════════════════════════════════════════════
// Legal categories pass
// ═══════════════════════════════════════════════════════════

const legalCategories = ['landmine', 'frustration'];
for (const cat of legalCategories) {
  test(`rule_selections: legal category accepted: "${cat}"`, () => {
    db.prepare("DELETE FROM rule_selections WHERE rule_id = 'rule-cat-test'").run();
    if (!tryExec(
      `INSERT INTO rule_selections (scan_job_id, rule_id, rule_version, effective_category, alert_level, enabled)
       VALUES (?, ?, ?, ?, ?, ?)`,
      'test-job', 'rule-cat-test', 1, cat, 'medium', 1
    )) {
      throw new Error(`legal category rejected: "${cat}"`);
    }
  });
}

// ═══════════════════════════════════════════════════════════
// Legal alert_levels pass
// ═══════════════════════════════════════════════════════════

const legalAlertLevels = ['critical', 'high', 'medium', 'low', 'info'];
for (const level of legalAlertLevels) {
  test(`rule_selections: legal alert_level accepted: "${level}"`, () => {
    db.prepare("DELETE FROM rule_selections WHERE rule_id = 'rule-al-test'").run();
    if (!tryExec(
      `INSERT INTO rule_selections (scan_job_id, rule_id, rule_version, effective_category, alert_level, enabled)
       VALUES (?, ?, ?, ?, ?, ?)`,
      'test-job', 'rule-al-test', 1, 'landmine', level, 1
    )) {
      throw new Error(`legal alert_level rejected: "${level}"`);
    }
  });
}

// ═══════════════════════════════════════════════════════════
// Item 五: Composite FK tests (findings -> rule_selections)
// ═══════════════════════════════════════════════════════════

test('findings: composite FK rejects rule not in rule_selections', () => {
  if (tryExec(
    `INSERT INTO findings (id, scan_job_id, rule_id, rule_version, effective_category, alert_level, status, confidence_bps, rationale, source_chapter_id, source_locator_json, provider_kind_snapshot, model_id_snapshot, created_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    'test-fk-fail', 'test-job', 'rule-not-in-selections', 1, 'landmine', 'medium', 'suspected', 5000, 'test', 'test-ch', '{}', 'openai', 'gpt-test', 0
  )) {
    throw new Error('finding with rule not in rule_selections was not rejected (composite FK)');
  }
});

test('findings: composite FK accepts rule in rule_selections', () => {
  if (!tryExec(
    `INSERT INTO findings (id, scan_job_id, rule_id, rule_version, effective_category, alert_level, status, confidence_bps, rationale, source_chapter_id, source_locator_json, provider_kind_snapshot, model_id_snapshot, created_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    'test-fk-ok', 'test-job', 'rule-fk', 1, 'landmine', 'medium', 'suspected', 5000, 'test', 'test-ch', '{}', 'openai', 'gpt-test', 0
  )) {
    throw new Error('valid finding with rule in rule_selections was rejected');
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Trigger: trg_findings_rule_version_insert
// ═══════════════════════════════════════════════════════════

test('findings INSERT: wrong rule_version rejected (trg_findings_rule_version_insert)', () => {
  try {
    db.exec(`
      INSERT INTO findings (id, scan_job_id, rule_id, rule_version, effective_category, alert_level, status, confidence_bps, rationale, source_chapter_id, source_locator_json, provider_kind_snapshot, model_id_snapshot, created_at_ms)
      VALUES ('test-fk-ver', 'test-job', 'rule-fk', 999, 'landmine', 'medium', 'suspected', 5000, 'test', 'test-ch', '{}', 'openai', 'gpt-test', 0)
    `);
    throw new Error('finding with wrong rule_version was not rejected');
  } catch (e) {
    if (!e.message.includes('finding rule_version must match rule_selections')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Trigger: trg_findings_chapter_book_insert (cross-book)
// ═══════════════════════════════════════════════════════════

test('findings INSERT: cross-book chapter rejected (trg_findings_chapter_book_insert)', () => {
  try {
    db.exec(`
      INSERT INTO findings (id, scan_job_id, rule_id, rule_version, effective_category, alert_level, status, confidence_bps, rationale, source_chapter_id, source_locator_json, provider_kind_snapshot, model_id_snapshot, created_at_ms)
      VALUES ('test-crossbook', 'test-job', 'rule-crossbook', 1, 'landmine', 'medium', 'suspected', 5000, 'test', 'test-ch-2', '{}', 'openai', 'gpt-test', 0)
    `);
    throw new Error('cross-book chapter finding was not rejected');
  } catch (e) {
    if (!e.message.includes('finding chapter must belong to same book as scan job')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Trigger: trg_findings_category_match_insert
// ═══════════════════════════════════════════════════════════

test('findings INSERT: wrong effective_category rejected (trg_findings_category_match_insert)', () => {
  try {
    // rule-cat-mismatch has effective_category='frustration', but we try 'landmine'
    db.exec(`
      INSERT INTO findings (id, scan_job_id, rule_id, rule_version, effective_category, alert_level, status, confidence_bps, rationale, source_chapter_id, source_locator_json, provider_kind_snapshot, model_id_snapshot, created_at_ms)
      VALUES ('test-cat-mis', 'test-job', 'rule-cat-mismatch', 1, 'landmine', 'medium', 'suspected', 5000, 'test', 'test-ch', '{}', 'openai', 'gpt-test', 0)
    `);
    throw new Error('finding with wrong effective_category was not rejected');
  } catch (e) {
    if (!e.message.includes('finding effective_category must match rule_selections')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Trigger: trg_findings_alert_level_match_insert
// ═══════════════════════════════════════════════════════════

test('findings INSERT: wrong alert_level rejected (trg_findings_alert_level_match_insert)', () => {
  try {
    // rule-al-mismatch has alert_level='high', but we try 'low'
    db.exec(`
      INSERT INTO findings (id, scan_job_id, rule_id, rule_version, effective_category, alert_level, status, confidence_bps, rationale, source_chapter_id, source_locator_json, provider_kind_snapshot, model_id_snapshot, created_at_ms)
      VALUES ('test-al-mis', 'test-job', 'rule-al-mismatch', 1, 'landmine', 'low', 'suspected', 5000, 'test', 'test-ch', '{}', 'openai', 'gpt-test', 0)
    `);
    throw new Error('finding with wrong alert_level was not rejected');
  } catch (e) {
    if (!e.message.includes('finding alert_level must match rule_selections')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Illegal finding status rejected
// ═══════════════════════════════════════════════════════════

const badStatuses = ['SUSPECTED', 'pending', 'Confirmed', 'REJECTED', '', 'new'];
for (const status of badStatuses) {
  test(`findings INSERT: illegal status rejected: "${status}"`, () => {
    if (tryExec(
      `INSERT INTO findings (id, scan_job_id, rule_id, rule_version, effective_category, alert_level, status, confidence_bps, rationale, source_chapter_id, source_locator_json, provider_kind_snapshot, model_id_snapshot, created_at_ms)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
      'find-bad-status', 'test-job', 'rule-fk', 1, 'landmine', 'medium', status, 5000, 'test reason', 'test-ch', '{}', 'openai', 'gpt-test', 0
    )) {
      throw new Error(`illegal finding status not rejected: "${status}"`);
    }
  });
}

// ═══════════════════════════════════════════════════════════
// Legal finding statuses pass
// ═══════════════════════════════════════════════════════════

const legalStatuses = ['suspected', 'pending_confirmation', 'confirmed', 'rejected'];
for (const status of legalStatuses) {
  test(`findings INSERT: legal status accepted: "${status}"`, () => {
    if (!tryExec(
      `INSERT INTO findings (id, scan_job_id, rule_id, rule_version, effective_category, alert_level, status, confidence_bps, rationale, source_chapter_id, source_locator_json, provider_kind_snapshot, model_id_snapshot, created_at_ms)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
      'find-' + status, 'test-job', 'rule-fk', 1, 'landmine', 'medium', status, 5000, 'test reason', 'test-ch', '{}', 'openai', 'gpt-test', 0
    )) {
      throw new Error(`legal finding status rejected: "${status}"`);
    }
  });
}

// ═══════════════════════════════════════════════════════════
// Item 五: Trigger: trg_findings_chapter_book_update (cross-book UPDATE)
// ═══════════════════════════════════════════════════════════

test('findings UPDATE: change source_chapter_id to cross-book rejected (trg_findings_chapter_book_update)', () => {
  // test-fk-ok has source_chapter_id='test-ch' (book test-book)
  try {
    db.exec("UPDATE findings SET source_chapter_id = 'test-ch-2' WHERE id = 'test-fk-ok'");
    throw new Error('cross-book chapter UPDATE was not rejected');
  } catch (e) {
    if (!e.message.includes('finding chapter must belong to same book as scan job')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Trigger: trg_findings_rule_version_update
// ═══════════════════════════════════════════════════════════

test('findings UPDATE: change rule_version to mismatch rejected (trg_findings_rule_version_update)', () => {
  try {
    db.exec("UPDATE findings SET rule_version = 999 WHERE id = 'test-fk-ok'");
    throw new Error('rule_version UPDATE to mismatch was not rejected');
  } catch (e) {
    if (!e.message.includes('finding rule_version must match rule_selections')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Trigger: trg_findings_category_match_update
// ═══════════════════════════════════════════════════════════

test('findings UPDATE: change effective_category to mismatch rejected (trg_findings_category_match_update)', () => {
  try {
    db.exec("UPDATE findings SET effective_category = 'frustration' WHERE id = 'test-fk-ok'");
    throw new Error('effective_category UPDATE to mismatch was not rejected');
  } catch (e) {
    if (!e.message.includes('finding effective_category must match rule_selections')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Trigger: trg_findings_alert_level_match_update
// ═══════════════════════════════════════════════════════════

test('findings UPDATE: change alert_level to mismatch rejected (trg_findings_alert_level_match_update)', () => {
  try {
    db.exec("UPDATE findings SET alert_level = 'high' WHERE id = 'test-fk-ok'");
    throw new Error('alert_level UPDATE to mismatch was not rejected');
  } catch (e) {
    if (!e.message.includes('finding alert_level must match rule_selections')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Trigger: trg_rule_selections_no_update_with_findings
// ═══════════════════════════════════════════════════════════

test('rule_selections UPDATE after findings exist rejected (trg_rule_selections_no_update_with_findings)', () => {
  // test-fk-ok is a finding for rule-fk, so rule-fk in rule_selections cannot be modified
  try {
    db.exec("UPDATE rule_selections SET enabled = 0 WHERE scan_job_id = 'test-job' AND rule_id = 'rule-fk'");
    throw new Error('rule_selections UPDATE after findings exist was not rejected');
  } catch (e) {
    if (!e.message.includes('cannot modify rule selection after findings exist')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Trigger: trg_chapters_book_id_immutable
// ═══════════════════════════════════════════════════════════

test('chapters UPDATE book_id rejected (trg_chapters_book_id_immutable)', () => {
  try {
    db.exec("UPDATE chapters SET book_id = 'test-book-2' WHERE id = 'test-ch'");
    throw new Error('chapters book_id UPDATE was not rejected');
  } catch (e) {
    if (!e.message.includes('cannot reassign chapter to different book')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Trigger: trg_evidence_chapter_book_insert (cross-book evidence INSERT)
// ═══════════════════════════════════════════════════════════

test('evidence INSERT: cross-book chapter rejected (trg_evidence_chapter_book_insert)', () => {
  // test-fk-ok is a finding for test-job (book test-book) with source_chapter_id=test-ch (book test-book)
  // Trying to insert evidence with chapter_id=test-ch-2 (book test-book-2) should fail
  try {
    db.exec(`
      INSERT INTO evidence (id, finding_id, ordinal, chapter_id, utf8_byte_start, utf8_byte_end, line_start, line_end, exact_quote, quote_hash, chapter_content_hash, source_locator_json, created_at_ms)
      VALUES ('ev-crossbook', 'test-fk-ok', 0, 'test-ch-2', 0, 5, 1, 1, 'hello', 'hash01', 'chash01', '{}', 0)
    `);
    throw new Error('cross-book evidence INSERT was not rejected');
  } catch (e) {
    if (!e.message.includes('evidence chapter must belong to same book as finding')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Valid evidence INSERT (same book)
// ═══════════════════════════════════════════════════════════

test('evidence INSERT: same-book chapter succeeds', () => {
  if (!tryExec(
    `INSERT INTO evidence (id, finding_id, ordinal, chapter_id, utf8_byte_start, utf8_byte_end, line_start, line_end, exact_quote, quote_hash, chapter_content_hash, source_locator_json, created_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    'ev-ok', 'test-fk-ok', 0, 'test-ch', 0, 5, 1, 1, 'hello', 'hash01', 'chash01', '{}', 0
  )) {
    throw new Error('same-book evidence INSERT failed');
  }
});

// ═══════════════════════════════════════════════════════════
// Item 五: Trigger: trg_evidence_chapter_book_update (cross-book evidence UPDATE)
// ═══════════════════════════════════════════════════════════

test('evidence UPDATE: cross-book chapter rejected (trg_evidence_chapter_book_update)', () => {
  try {
    db.exec("UPDATE evidence SET chapter_id = 'test-ch-2' WHERE id = 'ev-ok'");
    throw new Error('cross-book evidence UPDATE was not rejected');
  } catch (e) {
    if (!e.message.includes('evidence chapter must belong to same book as finding')) {
      throw new Error(`wrong error message: ${e.message}`);
    }
  }
});

// ═══════════════════════════════════════════════════════════
// Summary
// ═══════════════════════════════════════════════════════════

db.close();

if (passCount === testCount) {
  console.log(`\nMIGRATION VALIDATE OK (${passCount}/${testCount} passed)`);
} else {
  console.error(`\nMIGRATION VALIDATE FAIL (${passCount}/${testCount} passed)`);
  process.exit(1);
}
