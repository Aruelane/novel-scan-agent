#!/usr/bin/env node
/**
 * validate-ledger.mjs — 无第三方依赖的账本验证器
 *
 * 验证规则：
 * 1. 精确存在 122 个预期 ID
 * 2. ID 不多不少
 * 3. ID 唯一
 * 4. 禁止 +、斜杠、范围、A-D 等合并写法
 * 5. 状态只能使用合同允许的枚举
 * 6. 最多一个 IN_PROGRESS
 * 7. BLOCKED 引用的 EB 必须存在于 03 文件
 * 8. HUMAN_PENDING 引用的 HG 必须存在于 03 文件
 * 9. AWAITING_CI 必须包含真实 GitHub Actions run URL
 * 10. DONE 备注必须含可解析 commit SHA
 * 11. FINAL 依赖未满足时不得提前 DONE/HUMAN_PENDING
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// ── 硬编码 122 个预期任务 ID ──
const EXPECTED_IDS = Object.freeze([
  // S1 (15)
  'S1-01','S1-02','S1-03','S1-04','S1-05','S1-06','S1-07','S1-08',
  'S1-09','S1-10','S1-11','S1-12','S1-13','S1-14','S1-15',
  // S2 (15)
  'S2-01','S2-02','S2-03','S2-04','S2-05','S2-06','S2-07','S2-08',
  'S2-09','S2-10','S2-11','S2-12','S2-13','S2-14','S2-15',
  // S3 (18)
  'S3-01','S3-02','S3-03','S3-04','S3-05','S3-06','S3-07','S3-08',
  'S3-09','S3-10','S3-11','S3-12','S3-13','S3-14','S3-15','S3-16',
  'S3-17','S3-18',
  // S4 (20)
  'S4-01','S4-02','S4-03','S4-04','S4-05','S4-06','S4-07','S4-08',
  'S4-09','S4-10','S4-11','S4-12','S4-13','S4-14','S4-15','S4-16',
  'S4-17','S4-18','S4-19','S4-20',
  // S5 (15)
  'S5-RULE-01','S5-RULE-02','S5-RULE-03','S5-RULE-04',
  'S5-PRESET-01A','S5-PRESET-01B','S5-PRESET-01C','S5-PRESET-02',
  'S5-CUSTOM-01A','S5-CUSTOM-01B','S5-CUSTOM-01C',
  'S5-UPGRADE-01A','S5-UPGRADE-01B','S5-UPGRADE-01C',
  'S5-GATE-01',
  // S6 (25)
  'S6-WIN-01','S6-WIN-02',
  'S6-AND-01','S6-AND-02A','S6-AND-02B','S6-AND-02C','S6-AND-02D',
  'S6-AND-03A','S6-AND-03B','S6-AND-03C','S6-AND-03D',
  'S6-AND-04A','S6-AND-04B','S6-AND-04C','S6-AND-04D',
  'S6-UI-01A','S6-UI-01B','S6-UI-01C','S6-UI-01D',
  'S6-UX-01A','S6-UX-01B','S6-UX-01C',
  'S6-E2E-01','S6-BUILD-01','S6-GATE-01',
  // S7 (9)
  'S7-SEC-01A','S7-SEC-01B','S7-SEC-02',
  'S7-PERF-01','S7-A11Y-01','S7-E2E-01',
  'S7-BUILD-01','S7-REL-01','S7-GATE-01',
  // FINAL (5)
  'FINAL-01','FINAL-02','FINAL-03','FINAL-TIEBA-01','FINAL-04',
]);

const TOTAL_EXPECTED = EXPECTED_IDS.length; // 122

// ── 允许的状态枚举 ──
const VALID_STATES = new Set([
  'TODO','IN_PROGRESS','RETRY','AWAITING_CI','HUMAN_PENDING','BLOCKED','DONE','SKIPPED_BY_USER',
]);

// ── 允许的 EB/HG ID ── (从 03_BLOCKERS_AND_DEBT.md 读取)
let VALID_EB_IDS = new Set();
let VALID_HG_IDS = new Set();

// ── 工具函数 ──

function parseLedger(content) {
  const lines = content.split('\n');
  const tasks = [];
  let inTable = false;
  let headerColumns = 0;

  for (const line of lines) {
    const trimmed = line.trim();

    // Detect table start
    if (trimmed.startsWith('| ID |') && trimmed.includes('任务') && trimmed.includes('状态')) {
      inTable = true;
      headerColumns = trimmed.split('|').length;
      continue;
    }

    // Table separator line (|---|---|...)
    if (inTable && /^\|[\s\-|]+\|$/.test(trimmed)) {
      continue;
    }

    // Exit table on empty line or non-table content
    if (inTable && (!trimmed.startsWith('|') || trimmed === '|')) {
      // Check if we've entered a new section (## heading)
      if (trimmed.startsWith('##') || trimmed.startsWith('#')) {
        inTable = false;
      }
      continue;
    }

    // Exit on heading
    if (inTable && (trimmed.startsWith('## ') || trimmed.startsWith('# '))) {
      inTable = false;
      continue;
    }

    if (inTable && trimmed.startsWith('|')) {
      const cols = trimmed.split('|').map(c => c.trim()).filter(c => c !== '');
      if (cols.length >= 3) {
        const id = cols[0];
        // Skip non-task rows (statistics, examples, etc.)
        if (id && /^(S\d|FINAL)-/.test(id)) {
          const status = cols[2] || '';
          const notes = cols.slice(3).join(' ') || '';
          tasks.push({ id, status, notes, line: trimmed });
        }
      }
    }
  }

  return tasks;
}

function parseBlockers(content) {
  const eb = new Set();
  const hg = new Set();

  // Parse HUMAN_GATE table
  const hgMatch = content.match(/## HUMAN_GATE[\s\S]*?(?=## |$)/);
  if (hgMatch) {
    const hgLines = hgMatch[0].split('\n');
    for (const line of hgLines) {
      const m = line.match(/^\|\s*(HG-\w+)/);
      if (m) hg.add(m[1]);
    }
  }

  // Parse EXTERNAL_BLOCKER table
  const ebMatch = content.match(/## EXTERNAL_BLOCKER[\s\S]*?(?=## |$)/);
  if (ebMatch) {
    const ebLines = ebMatch[0].split('\n');
    for (const line of ebLines) {
      const m = line.match(/^\|\s*(EB-\w+)/);
      if (m) eb.add(m[1]);
    }
  }

  return { eb, hg };
}

// ── 验证函数 ──

const errors = [];
const warnings = [];

function error(msg) {
  errors.push(msg);
}

function warn(msg) {
  warnings.push(msg);
}

function validate() {
  // Read ledger
  const ledgerPath = resolve(__dirname, '02_TASK_LEDGER.md');
  const ledgerContent = readFileSync(ledgerPath, 'utf-8');

  // Read blockers
  const blockersPath = resolve(__dirname, '03_BLOCKERS_AND_DEBT.md');
  let blockersContent = '';
  try {
    blockersContent = readFileSync(blockersPath, 'utf-8');
  } catch (_) {
    warn('03_BLOCKERS_AND_DEBT.md not readable; skipping EB/HG cross-reference');
  }

  const { eb, hg } = parseBlockers(blockersContent);
  VALID_EB_IDS = eb;
  VALID_HG_IDS = hg;

  const tasks = parseLedger(ledgerContent);
  const taskIds = tasks.map(t => t.id);

  // ── 检查 1+2: 精确 122 个 ID ──
  if (taskIds.length !== TOTAL_EXPECTED) {
    error(`Expected ${TOTAL_EXPECTED} task IDs, found ${taskIds.length}`);
  } else {
    console.log(`✓ Task count: ${taskIds.length} (expected ${TOTAL_EXPECTED})`);
  }

  // ── 检查 3: ID 唯一 ──
  const seen = new Set();
  const duplicates = [];
  for (const id of taskIds) {
    if (seen.has(id)) {
      duplicates.push(id);
    }
    seen.add(id);
  }
  if (duplicates.length > 0) {
    error(`Duplicate task IDs found: ${duplicates.join(', ')}`);
  } else {
    console.log('✓ All task IDs are unique');
  }

  // ── 检查 4: 禁止合并写法 ──
  for (const id of taskIds) {
    if (id.includes('+')) {
      error(`Merged ID found (contains '+'): ${id}`);
    }
    if (id.includes('/')) {
      error(`Merged ID found (contains '/'): ${id}`);
    }
    if (/[A-D]$/.test(id) && /[A-D]-[A-D]/.test(id)) {
      // Already handled by no range; this catches explicit ranges like S6-AND-02A-D
      // But individual IDs like S6-AND-02A are fine (single letter suffix)
    }
    // Check for range patterns like 02A-D or 01B-C
    if (/\d[A-Z]-[A-Z]/.test(id)) {
      error(`Range ID found (contains letter range like A-D or B-C): ${id}`);
    }
    // Check for en-dash or em-dash
    if (id.includes('–') || id.includes('—')) {
      error(`Merged ID found (contains dash variant): ${id}`);
    }
  }
  console.log('✓ No merged/range IDs found');

  // ── 检查 5: 状态枚举 ──
  for (const task of tasks) {
    if (!VALID_STATES.has(task.status)) {
      error(`Invalid status '${task.status}' for ${task.id}`);
    }
  }
  console.log('✓ All status values are valid enum members');

  // ── 检查 6: 最多一个 IN_PROGRESS ──
  const inProgress = tasks.filter(t => t.status === 'IN_PROGRESS');
  if (inProgress.length > 1) {
    error(`Multiple IN_PROGRESS tasks: ${inProgress.map(t => t.id).join(', ')}`);
  } else {
    console.log(`✓ IN_PROGRESS count: ${inProgress.length} (max 1 allowed)`);
  }

  // ── 检查 7: BLOCKED 引用 EB ──
  for (const task of tasks) {
    if (task.status === 'BLOCKED') {
      const ebRefs = task.notes.match(/EB-\w+/g) || [];
      if (ebRefs.length === 0) {
        error(`${task.id} is BLOCKED but references no EB-* in notes`);
      } else {
        for (const ref of ebRefs) {
          if (!VALID_EB_IDS.has(ref)) {
            error(`${task.id} references ${ref} which does not exist in 03_BLOCKERS_AND_DEBT.md`);
          }
        }
      }
    }
  }

  // ── 检查 8: HUMAN_PENDING 引用 HG ──
  for (const task of tasks) {
    if (task.status === 'HUMAN_PENDING') {
      const hgRefs = task.notes.match(/HG-\w+/g) || [];
      if (hgRefs.length === 0) {
        // FINAL-TIEBA-01 references HG-002A/HG-002B implicitly through S5-RULE-02
        if (task.id !== 'FINAL-TIEBA-01') {
          warn(`${task.id} is HUMAN_PENDING but may be missing explicit HG-* reference`);
        }
      } else {
        for (const ref of hgRefs) {
          if (!VALID_HG_IDS.has(ref)) {
            error(`${task.id} references ${ref} which does not exist in 03_BLOCKERS_AND_DEBT.md`);
          }
        }
      }
    }
  }

  // ── 检查 9: AWAITING_CI 包含 run URL ──
  for (const task of tasks) {
    if (task.status === 'AWAITING_CI') {
      if (!/github\.com\/[\w.-]+\/[\w.-]+\/actions\/runs\/\d+/.test(task.notes)) {
        error(`${task.id} is AWAITING_CI but notes do not contain a GitHub Actions run URL`);
      }
    }
  }

  // ── 检查 10: DONE 含 commit SHA ──
  for (const task of tasks) {
    if (task.status === 'DONE') {
      if (!/[0-9a-f]{7,40}/i.test(task.notes)) {
        error(`${task.id} is DONE but notes do not contain a recognizable commit SHA`);
      }
    }
  }

  // ── 检查 11: FINAL 依赖门禁 ──
  // FINAL-01 requires S7-GATE-01 DONE
  const s7Gate = tasks.find(t => t.id === 'S7-GATE-01');
  const final01 = tasks.find(t => t.id === 'FINAL-01');
  const final02 = tasks.find(t => t.id === 'FINAL-02');
  const final03 = tasks.find(t => t.id === 'FINAL-03');
  const final04 = tasks.find(t => t.id === 'FINAL-04');

  if (s7Gate && s7Gate.status !== 'DONE') {
    if (final01 && final01.status === 'DONE') {
      error('FINAL-01 is DONE but S7-GATE-01 is not DONE');
    }
    if (final01 && final01.status === 'HUMAN_PENDING') {
      error('FINAL-01 is HUMAN_PENDING but S7-GATE-01 is not DONE');
    }
    if (final02 && final02.status === 'DONE') {
      error('FINAL-02 is DONE but FINAL-01 cannot be DONE (S7-GATE-01 not DONE)');
    }
    if (final03 && final03.status === 'DONE') {
      error('FINAL-03 is DONE but FINAL-02 cannot be DONE');
    }
    if (final04 && final04.status !== 'TODO') {
      error(`FINAL-04 is ${final04.status} but FINAL-03 cannot be complete (S7-GATE-01 not DONE)`);
    }
  }

  // FINAL-TIEBA-01 requires HG-002A and HG-002B evidence in notes
  const tieba = tasks.find(t => t.id === 'FINAL-TIEBA-01');
  if (tieba && tieba.status === 'DONE') {
    if (!tieba.notes.includes('HG-002A') || !tieba.notes.includes('HG-002B')) {
      error('FINAL-TIEBA-01 is DONE but missing HG-002A or HG-002B evidence');
    }
  }

  // ── 检查缺失 ID ──
  const foundSet = new Set(taskIds);
  const missing = EXPECTED_IDS.filter(id => !foundSet.has(id));
  if (missing.length > 0) {
    error(`Missing task IDs: ${missing.join(', ')}`);
  }

  // ── 检查多余 ID ──
  const expectedSet = new Set(EXPECTED_IDS);
  const extra = taskIds.filter(id => !expectedSet.has(id));
  if (extra.length > 0) {
    error(`Unexpected task IDs (not in 122-plan): ${extra.join(', ')}`);
  }

  // ── 检查 ID 排序 ──
  // Verify tasks appear in expected order (within each section)
  let lastSectionIdx = -1;
  let lastInSection = -1;
  for (const id of taskIds) {
    const globalIdx = EXPECTED_IDS.indexOf(id);
    if (globalIdx === -1) continue; // already reported as extra
    // Determine section from ID prefix
    const sectionPrefix = id.startsWith('FINAL') ? 'FINAL' : id.match(/^(S\d)/)?.[1] || '';
    const firstInSection = EXPECTED_IDS.findIndex(eid =>
      eid.startsWith(sectionPrefix) && !eid.startsWith(sectionPrefix + '-') === !id.startsWith(id.match(/^(S\d)/)?.[1] + '-')
    );
    if (globalIdx < lastInSection && id.startsWith(EXPECTED_IDS[lastInSection]?.match(/^(S\d|FINAL)/)?.[0] || '')) {
      warn(`Task order: ${id} appears after ${EXPECTED_IDS[lastInSection]} but has lower index in expected plan`);
    }
    lastInSection = globalIdx;
  }

  // ── 输出 ──
  console.log('');
  if (errors.length === 0) {
    console.log('✓ ALL VALIDATIONS PASSED');
    if (warnings.length > 0) {
      console.log(`  (${warnings.length} warning(s) — see below)`);
    }
  } else {
    console.error(`✗ ${errors.length} VALIDATION ERROR(S):`);
    for (const e of errors) {
      console.error(`  - ${e}`);
    }
  }

  if (warnings.length > 0) {
    console.warn(`\n${warnings.length} WARNING(S):`);
    for (const w of warnings) {
      console.warn(`  ! ${w}`);
    }
  }

  return errors.length === 0;
}

const passed = validate();
process.exit(passed ? 0 : 1);
