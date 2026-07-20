-- Runtime API credentials are deliberately absent from this schema.
-- `credential_ref` is only an opaque handle into a platform secret store. It
-- must never contain an API key, bearer token, password, or custom auth header.

CREATE TABLE provider_profiles (
    id TEXT PRIMARY KEY NOT NULL
        CHECK (length(trim(id)) > 0),
    provider_kind TEXT NOT NULL
        CHECK (length(trim(provider_kind)) > 0),
    display_name TEXT NOT NULL
        CHECK (length(trim(display_name)) > 0),
    base_url TEXT,
    default_model_id TEXT,
    credential_ref TEXT
        CONSTRAINT ck_provider_credential_ref CHECK (
            credential_ref IS NULL
            OR (
                typeof(credential_ref) = 'text'
                AND length(credential_ref) >= length('secret-ref:x')
                AND substr(credential_ref, 1, 11) = 'secret-ref:'
                AND credential_ref = trim(credential_ref)
                AND instr(credential_ref, char(10)) = 0
                AND instr(credential_ref, char(13)) = 0
                AND instr(credential_ref, char(9)) = 0
                AND instr(credential_ref, char(0)) = 0
                AND length(substr(credential_ref, 12)) >= 1
                AND substr(credential_ref, 12) NOT GLOB '*[^a-zA-Z0-9._-]*'
                AND length(credential_ref) <= 256
            )
        ),
    credential_state TEXT NOT NULL DEFAULT 'missing'
        CHECK (credential_state IN ('missing', 'configured', 'unavailable')),
    enabled INTEGER NOT NULL DEFAULT 1
        CHECK (enabled IN (0, 1)),
    created_at_ms INTEGER NOT NULL
        CHECK (created_at_ms >= 0),
    updated_at_ms INTEGER NOT NULL
        CHECK (updated_at_ms >= created_at_ms),
    CHECK (
        credential_state <> 'configured'
        OR credential_ref IS NOT NULL
    )
);

CREATE TABLE books (
    id TEXT PRIMARY KEY NOT NULL
        CHECK (length(trim(id)) > 0),
    title TEXT NOT NULL
        CHECK (length(trim(title)) > 0),
    source_display_name TEXT NOT NULL
        CHECK (length(trim(source_display_name)) > 0),
    source_uri TEXT,
    source_format TEXT NOT NULL
        CHECK (source_format IN (
            'plain_text', 'markdown', 'epub', 'docx', 'pdf', 'html',
            'mobi', 'azw3', 'archive', 'other'
        )),
    source_size_bytes INTEGER
        CHECK (source_size_bytes IS NULL OR source_size_bytes >= 0),
    fingerprint TEXT NOT NULL
        CHECK (length(trim(fingerprint)) > 0),
    imported_at_ms INTEGER NOT NULL
        CHECK (imported_at_ms >= 0),
    updated_at_ms INTEGER NOT NULL
        CHECK (updated_at_ms >= imported_at_ms)
);

CREATE TABLE chapters (
    id TEXT PRIMARY KEY NOT NULL
        CHECK (length(trim(id)) > 0),
    book_id TEXT NOT NULL
        REFERENCES books(id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL
        CHECK (ordinal >= 0),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    content_hash TEXT NOT NULL
        CHECK (length(trim(content_hash)) > 0),
    source_locator_json TEXT NOT NULL,
    character_count INTEGER NOT NULL
        CHECK (character_count >= 0),
    created_at_ms INTEGER NOT NULL
        CHECK (created_at_ms >= 0),
    UNIQUE (book_id, ordinal)
);

CREATE TABLE scan_jobs (
    id TEXT PRIMARY KEY NOT NULL
        CHECK (length(trim(id)) > 0),
    book_id TEXT NOT NULL
        REFERENCES books(id) ON DELETE CASCADE,
    status TEXT NOT NULL
        CHECK (status IN ('pending', 'running', 'paused', 'completed', 'failed')),
    provider_profile_id TEXT
        REFERENCES provider_profiles(id) ON DELETE SET NULL,
    provider_kind_snapshot TEXT NOT NULL
        CHECK (length(trim(provider_kind_snapshot)) > 0),
    model_id_snapshot TEXT NOT NULL
        CHECK (length(trim(model_id_snapshot)) > 0),
    rule_pack_id_snapshot TEXT NOT NULL
        CHECK (length(trim(rule_pack_id_snapshot)) > 0),
    rule_pack_version_snapshot TEXT NOT NULL
        CHECK (length(trim(rule_pack_version_snapshot)) > 0),
    context_budget_chars INTEGER NOT NULL DEFAULT 8000
        CHECK (context_budget_chars > 0),
    retain_unverified_candidates INTEGER NOT NULL DEFAULT 1
        CHECK (retain_unverified_candidates IN (0, 1)),
    current_chapter_position INTEGER NOT NULL DEFAULT 0
        CHECK (current_chapter_position >= 0),
    total_chapters INTEGER NOT NULL DEFAULT 0
        CHECK (total_chapters >= 0),
    created_at_ms INTEGER NOT NULL
        CHECK (created_at_ms >= 0),
    updated_at_ms INTEGER NOT NULL
        CHECK (updated_at_ms >= created_at_ms),
    started_at_ms INTEGER
        CHECK (started_at_ms IS NULL OR started_at_ms >= created_at_ms),
    completed_at_ms INTEGER
        CHECK (completed_at_ms IS NULL OR completed_at_ms >= created_at_ms),
    last_error TEXT,
    CHECK (current_chapter_position <= total_chapters)
);

CREATE TABLE rule_selections (
    scan_job_id TEXT NOT NULL
        REFERENCES scan_jobs(id) ON DELETE CASCADE,
    rule_id TEXT NOT NULL
        CHECK (length(trim(rule_id)) > 0),
    rule_version INTEGER NOT NULL
        CHECK (rule_version >= 1),
    effective_category TEXT NOT NULL
        CHECK (effective_category IN ('landmine', 'frustration')),
    alert_level TEXT NOT NULL
        CHECK (alert_level IN ('critical', 'high', 'medium', 'low', 'info')),
    enabled INTEGER NOT NULL DEFAULT 1
        CHECK (enabled IN (0, 1)),
    PRIMARY KEY (scan_job_id, rule_id)
);

CREATE TABLE checkpoints (
    scan_job_id TEXT PRIMARY KEY NOT NULL
        REFERENCES scan_jobs(id) ON DELETE CASCADE,
    schema_version INTEGER NOT NULL
        CHECK (schema_version = 2),
    document_fingerprint TEXT NOT NULL
        CHECK (length(trim(document_fingerprint)) > 0),
    scan_profile_fingerprint TEXT NOT NULL
        CHECK (length(trim(scan_profile_fingerprint)) > 0),
    next_chapter_position INTEGER NOT NULL
        CHECK (next_chapter_position >= 0),
    processed_chapters_json TEXT NOT NULL,
    context_snapshot_json TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL
        CHECK (updated_at_ms >= 0)
);

CREATE TABLE findings (
    id TEXT PRIMARY KEY NOT NULL
        CHECK (length(trim(id)) > 0),
    scan_job_id TEXT NOT NULL
        REFERENCES scan_jobs(id) ON DELETE CASCADE,
    rule_id TEXT NOT NULL
        CHECK (length(trim(rule_id)) > 0),
    rule_version INTEGER NOT NULL
        CHECK (rule_version >= 1),
    effective_category TEXT NOT NULL
        CHECK (effective_category IN ('landmine', 'frustration')),
    alert_level TEXT NOT NULL
        CHECK (alert_level IN ('critical', 'high', 'medium', 'low', 'info')),
    status TEXT NOT NULL
        CHECK (status IN (
            'suspected', 'pending_confirmation', 'confirmed', 'rejected'
        )),
    confidence_bps INTEGER NOT NULL
        CHECK (confidence_bps BETWEEN 0 AND 10000),
    rationale TEXT NOT NULL,
    verification_note TEXT,
    source_chapter_id TEXT NOT NULL
        REFERENCES chapters(id) ON DELETE CASCADE,
    source_locator_json TEXT NOT NULL,
    provider_kind_snapshot TEXT NOT NULL
        CHECK (length(trim(provider_kind_snapshot)) > 0),
    model_id_snapshot TEXT NOT NULL
        CHECK (length(trim(model_id_snapshot)) > 0),
    created_at_ms INTEGER NOT NULL
        CHECK (created_at_ms >= 0),
    FOREIGN KEY (scan_job_id, rule_id) REFERENCES rule_selections(scan_job_id, rule_id)
);

CREATE TABLE evidence (
    id TEXT PRIMARY KEY NOT NULL
        CHECK (length(trim(id)) > 0),
    finding_id TEXT NOT NULL
        REFERENCES findings(id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL
        CHECK (ordinal >= 0),
    chapter_id TEXT NOT NULL
        REFERENCES chapters(id) ON DELETE CASCADE,
    utf8_byte_start INTEGER NOT NULL
        CHECK (utf8_byte_start >= 0),
    utf8_byte_end INTEGER NOT NULL
        CHECK (utf8_byte_end > utf8_byte_start),
    line_start INTEGER NOT NULL
        CHECK (line_start >= 1),
    line_end INTEGER NOT NULL
        CHECK (line_end >= line_start),
    exact_quote TEXT NOT NULL
        CHECK (length(exact_quote) > 0),
    quote_hash TEXT NOT NULL
        CHECK (length(trim(quote_hash)) > 0),
    chapter_content_hash TEXT NOT NULL
        CHECK (length(trim(chapter_content_hash)) > 0),
    source_locator_json TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
        CHECK (created_at_ms >= 0),
    UNIQUE (finding_id, ordinal)
);

CREATE TRIGGER trg_findings_chapter_book_insert
BEFORE INSERT ON findings
BEGIN
    SELECT RAISE(ABORT, 'finding chapter must belong to same book as scan job')
    WHERE (SELECT book_id FROM chapters WHERE id = NEW.source_chapter_id)
       != (SELECT book_id FROM scan_jobs WHERE id = NEW.scan_job_id);
END;

CREATE TRIGGER trg_findings_chapter_book_update
BEFORE UPDATE ON findings
BEGIN
    SELECT RAISE(ABORT, 'finding chapter must belong to same book as scan job')
    WHERE (SELECT book_id FROM chapters WHERE id = NEW.source_chapter_id)
       != (SELECT book_id FROM scan_jobs WHERE id = NEW.scan_job_id);
END;

CREATE TRIGGER trg_findings_rule_version_insert
BEFORE INSERT ON findings
BEGIN
    SELECT RAISE(ABORT, 'finding rule_version must match rule_selections')
    WHERE (SELECT rule_version FROM rule_selections
           WHERE scan_job_id = NEW.scan_job_id AND rule_id = NEW.rule_id)
       != NEW.rule_version;
END;

CREATE TRIGGER trg_findings_rule_version_update
BEFORE UPDATE ON findings
BEGIN
    SELECT RAISE(ABORT, 'finding rule_version must match rule_selections')
    WHERE (SELECT rule_version FROM rule_selections
           WHERE scan_job_id = NEW.scan_job_id AND rule_id = NEW.rule_id)
       != NEW.rule_version;
END;

CREATE TRIGGER trg_findings_category_match_insert
BEFORE INSERT ON findings
BEGIN
    SELECT RAISE(ABORT, 'finding effective_category must match rule_selections')
    WHERE (SELECT effective_category FROM rule_selections
           WHERE scan_job_id = NEW.scan_job_id AND rule_id = NEW.rule_id)
       != NEW.effective_category;
END;

CREATE TRIGGER trg_findings_category_match_update
BEFORE UPDATE ON findings
BEGIN
    SELECT RAISE(ABORT, 'finding effective_category must match rule_selections')
    WHERE (SELECT effective_category FROM rule_selections
           WHERE scan_job_id = NEW.scan_job_id AND rule_id = NEW.rule_id)
       != NEW.effective_category;
END;

CREATE TRIGGER trg_findings_alert_level_match_insert
BEFORE INSERT ON findings
BEGIN
    SELECT RAISE(ABORT, 'finding alert_level must match rule_selections')
    WHERE (SELECT alert_level FROM rule_selections
           WHERE scan_job_id = NEW.scan_job_id AND rule_id = NEW.rule_id)
       != NEW.alert_level;
END;

CREATE TRIGGER trg_findings_alert_level_match_update
BEFORE UPDATE ON findings
BEGIN
    SELECT RAISE(ABORT, 'finding alert_level must match rule_selections')
    WHERE (SELECT alert_level FROM rule_selections
           WHERE scan_job_id = NEW.scan_job_id AND rule_id = NEW.rule_id)
       != NEW.alert_level;
END;

CREATE TRIGGER trg_rule_selections_no_update_with_findings
BEFORE UPDATE ON rule_selections
WHEN EXISTS (SELECT 1 FROM findings WHERE findings.scan_job_id = OLD.scan_job_id AND findings.rule_id = OLD.rule_id)
BEGIN
    SELECT RAISE(ABORT, 'cannot modify rule selection after findings exist');
END;

CREATE TRIGGER trg_chapters_book_id_immutable
BEFORE UPDATE OF book_id ON chapters
BEGIN
    SELECT RAISE(ABORT, 'cannot reassign chapter to different book');
END;

CREATE TRIGGER trg_evidence_chapter_book_insert
BEFORE INSERT ON evidence
BEGIN
    SELECT RAISE(ABORT, 'evidence chapter must belong to same book as finding')
    WHERE (SELECT book_id FROM chapters WHERE id = NEW.chapter_id)
       != (
           SELECT book_id FROM chapters
           WHERE id = (SELECT source_chapter_id FROM findings WHERE id = NEW.finding_id)
       );
END;

CREATE TRIGGER trg_evidence_chapter_book_update
BEFORE UPDATE ON evidence
BEGIN
    SELECT RAISE(ABORT, 'evidence chapter must belong to same book as finding')
    WHERE (SELECT book_id FROM chapters WHERE id = NEW.chapter_id)
       != (
           SELECT book_id FROM chapters
           WHERE id = (SELECT source_chapter_id FROM findings WHERE id = NEW.finding_id)
       );
END;

CREATE TRIGGER trg_scan_jobs_immutable_after_checkpoint
BEFORE UPDATE ON scan_jobs
WHEN EXISTS (SELECT 1 FROM checkpoints WHERE checkpoints.scan_job_id = OLD.id)
BEGIN
    SELECT RAISE(ABORT, 'cannot modify scan_job fields after checkpoint exists')
    WHERE NEW.provider_kind_snapshot != OLD.provider_kind_snapshot
       OR NEW.model_id_snapshot != OLD.model_id_snapshot
       OR NEW.rule_pack_id_snapshot != OLD.rule_pack_id_snapshot
       OR NEW.rule_pack_version_snapshot != OLD.rule_pack_version_snapshot
       OR NEW.context_budget_chars != OLD.context_budget_chars
       OR NEW.retain_unverified_candidates != OLD.retain_unverified_candidates
       OR NEW.book_id IS NOT OLD.book_id
       OR NEW.provider_profile_id IS NOT OLD.provider_profile_id;
END;

CREATE TRIGGER trg_rule_selections_no_insert_after_checkpoint
BEFORE INSERT ON rule_selections
WHEN EXISTS (SELECT 1 FROM checkpoints WHERE checkpoints.scan_job_id = NEW.scan_job_id)
BEGIN
    SELECT RAISE(ABORT, 'cannot insert rule selection after checkpoint exists');
END;

CREATE TRIGGER trg_rule_selections_no_update_after_checkpoint
BEFORE UPDATE ON rule_selections
WHEN EXISTS (SELECT 1 FROM checkpoints WHERE checkpoints.scan_job_id = OLD.scan_job_id)
  OR EXISTS (SELECT 1 FROM checkpoints WHERE checkpoints.scan_job_id = NEW.scan_job_id)
BEGIN
    SELECT RAISE(ABORT, 'cannot update rule selection after checkpoint exists');
END;

CREATE TRIGGER trg_rule_selections_no_delete_after_checkpoint
BEFORE DELETE ON rule_selections
WHEN EXISTS (SELECT 1 FROM checkpoints WHERE checkpoints.scan_job_id = OLD.scan_job_id)
BEGIN
    SELECT RAISE(ABORT, 'cannot delete rule selection after checkpoint exists');
END;

-- S3 extensions: stop reason, usage budget, and usage tracking.
-- Applied as ALTER for pre-existing scan_jobs; safe for fresh installs too.

ALTER TABLE scan_jobs ADD COLUMN stop_reason TEXT
    CHECK (stop_reason IS NULL OR stop_reason IN (
        'completed', 'user_paused', 'user_cancelled', 'budget_reached', 'failed'
    ));

ALTER TABLE scan_jobs ADD COLUMN usage_budget_json TEXT;

-- Usage tracking for provider-neutral accounting.
-- Input/output units are provider-reported; requests are deterministic.
CREATE TABLE usage_events (
    id TEXT PRIMARY KEY NOT NULL
        CHECK (length(trim(id)) > 0),
    scan_job_id TEXT NOT NULL
        REFERENCES scan_jobs(id) ON DELETE CASCADE,
    chapter_id TEXT NOT NULL
        REFERENCES chapters(id) ON DELETE CASCADE,
    window_index INTEGER NOT NULL DEFAULT 0
        CHECK (window_index >= 0),
    attempt INTEGER NOT NULL DEFAULT 1
        CHECK (attempt >= 1),
    input_units INTEGER NOT NULL
        CHECK (input_units >= 0),
    output_units INTEGER NOT NULL
        CHECK (output_units >= 0),
    outcome TEXT NOT NULL
        CHECK (outcome IN ('success', 'retry', 'failed')),
    created_at_ms INTEGER NOT NULL
        CHECK (created_at_ms >= 0),
    FOREIGN KEY (scan_job_id, chapter_id) REFERENCES chapters(book_id, ordinal)
);

CREATE INDEX idx_usage_events_job
    ON usage_events(scan_job_id, created_at_ms);

CREATE TRIGGER trg_checkpoints_scan_job_id_immutable
BEFORE UPDATE OF scan_job_id ON checkpoints
BEGIN
    SELECT RAISE(ABORT, 'cannot reassign checkpoint to another scan job');
END;

CREATE INDEX idx_provider_profiles_enabled
    ON provider_profiles(enabled, provider_kind);

CREATE UNIQUE INDEX idx_books_fingerprint
    ON books(fingerprint);

CREATE INDEX idx_chapters_book_order
    ON chapters(book_id, ordinal);

CREATE INDEX idx_scan_jobs_book_status
    ON scan_jobs(book_id, status, updated_at_ms);

CREATE INDEX idx_rule_selections_enabled
    ON rule_selections(scan_job_id, enabled, effective_category, alert_level);

CREATE INDEX idx_findings_job_status
    ON findings(scan_job_id, status, effective_category, alert_level);

CREATE INDEX idx_findings_chapter
    ON findings(source_chapter_id);

CREATE INDEX idx_evidence_finding_order
    ON evidence(finding_id, ordinal);

CREATE INDEX idx_evidence_chapter_lines
    ON evidence(chapter_id, line_start, line_end);
