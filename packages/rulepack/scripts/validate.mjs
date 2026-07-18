import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { Ajv2020 } from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(here, "..");

async function readJson(relativePath) {
  const absolutePath = resolve(packageRoot, relativePath);
  const text = await readFile(absolutePath, "utf8");
  try {
    return JSON.parse(text);
  } catch (error) {
    throw new Error(`${relativePath} 不是合法 JSON: ${error.message}`);
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

// ── Load data ──

const [packSchema, selectionSchema, pack, selection] = await Promise.all([
  readJson("schemas/rulepack.schema.json"),
  readJson("schemas/rule-selection.schema.json"),
  readJson("packs/yy-novel-bar/2026.0.0-seed.1.json"),
  readJson("examples/rule-selection.example.json")
]);

// ── Schema self-checks ──

assert(packSchema.$schema?.includes("2020-12"), "规则包 Schema 必须使用 JSON Schema 2020-12");
assert(selectionSchema.$schema?.includes("2020-12"), "用户选择 Schema 必须使用 JSON Schema 2020-12");

// ── Ajv 2020-12 validation ──

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);

const validatePack = ajv.compile(packSchema);
if (!validatePack(pack)) {
  const errors = validatePack.errors
    .map((e) => `  ${e.instancePath}: ${e.message}`)
    .join("\n");
  throw new Error(`种子包未通过 JSON Schema 校验:\n${errors}`);
}

// The selection schema uses conditional required, which triggers Ajv strict mode.
// Compile with strictRequired disabled to accept this valid 2020-12 construct.
const ajvSelection = new Ajv2020({ allErrors: true, strictRequired: false });
addFormats(ajvSelection);
const validateSelection = ajvSelection.compile(selectionSchema);
if (!validateSelection(selection)) {
  const errors = validateSelection.errors
    .map((e) => `  ${e.instancePath}: ${e.message}`)
    .join("\n");
  throw new Error(`示例选择未通过 JSON Schema 校验:\n${errors}`);
}

// ── Business invariants ──

assert(pack.schemaVersion === "1.0.0", "当前种子包 schemaVersion 应为 1.0.0");

const expectedCounts = pack.communityContext.expectedCategoryCounts;
const counts = pack.rules.reduce(
  (result, rule) => {
    result[rule.category] = (result[rule.category] ?? 0) + 1;
    return result;
  },
  { landmine: 0, frustration: 0 }
);
assert(counts.landmine === expectedCounts.landmine,
  `雷点数量应为 ${expectedCounts.landmine}，实际为 ${counts.landmine}`);
assert(counts.frustration === expectedCounts.frustration,
  `郁闷点数量应为 ${expectedCounts.frustration}，实际为 ${counts.frustration}`);

const totalRules = counts.landmine + counts.frustration;
assert(totalRules === 32, `规则总数应为 32，实际为 ${totalRules}`);

const sourceIds = new Set(pack.sourceCatalog.map((source) => source.id));
assert(sourceIds.size === pack.sourceCatalog.length, "sourceCatalog.id 必须唯一");

const severityIds = new Set(pack.severityLevels.map((level) => level.id));
assert(severityIds.size === pack.severityLevels.length, "severityLevels.id 必须唯一");

const validCategories = new Set(["landmine", "frustration"]);
const profileIds = new Set(Object.keys(pack.detectionProfiles));
const ruleIds = new Set();

for (const rule of pack.rules) {
  assert(!ruleIds.has(rule.id), `重复规则 id: ${rule.id}`);
  ruleIds.add(rule.id);
  assert(validCategories.has(rule.category),
    `${rule.id} 的 category "${rule.category}" 不是 landmine 或 frustration`);
  assert(Number.isInteger(rule.version) && rule.version >= 1,
    `${rule.id} 的 version 必须是 >= 1 的整数`);
  assert(profileIds.has(rule.detection.profileRef),
    `${rule.id} 引用了不存在的 detection profile`);
  assert(severityIds.has(rule.defaultSeverity),
    `${rule.id} 的 defaultSeverity 不存在`);
  assert(rule.userConfig.toggleable === true,
    `${rule.id} 必须允许用户开关`);
  assert(rule.userConfig.severityOverride === true,
    `${rule.id} 必须允许用户覆写严重度`);
  assert(rule.detection.criteria.length > 0, `${rule.id} 缺少判据`);
  assert(rule.detection.exclusions.length > 0, `${rule.id} 缺少排除条件`);
  assert(rule.detection.pendingConditions.length > 0, `${rule.id} 缺少待确认条件`);
  assert(rule.provenance.sourceRefs.length > 0, `${rule.id} 缺少来源引用`);

  const validScopes = new Set(["local", "chapter", "cross_chapter", "whole_book"]);
  assert(validScopes.has(rule.detection.confirmationScope),
    `${rule.id} 的 confirmationScope "${rule.detection.confirmationScope}" 非法`);

  for (const sourceRef of rule.provenance.sourceRefs) {
    assert(sourceIds.has(sourceRef),
      `${rule.id} 引用了不存在的来源 ${sourceRef}`);
  }
  if (rule.status !== "verified") {
    assert(rule.defaultEnabled === false,
      `${rule.id} 尚未核验，必须默认关闭`);
  }
  if (rule.nameStatus === "placeholder") {
    assert(rule.detection.mode === "manual_only",
      `${rule.id} 是占位条目，只能 manual_only`);
  }
  if (rule.status === "verified") {
    assert(rule.provenance.verification === "verified",
      `${rule.id} 标记 verified 但来源未验证`);
  }
}

const crossCategory = pack.rules.filter(
  (rule) => rule.conceptId === "relationship.accepting-prior-partner"
);
assert(crossCategory.length === 2, "接盘概念应有雷点档和郁闷点档两个条目");
assert(
  new Set(crossCategory.map((rule) => rule.category)).size === 2,
  "接盘概念的两个条目必须分别属于 landmine 和 frustration"
);

assert(selection.packId === pack.id, "示例选择的 packId 与规则包不一致");
assert(selection.packVersion === pack.version, "示例选择的 packVersion 与规则包不一致");
const overrideIds = new Set();
for (const override of selection.overrides) {
  assert(ruleIds.has(override.ruleId),
    `示例选择引用了不存在的规则 ${override.ruleId}`);
  assert(!overrideIds.has(override.ruleId),
    `示例选择重复覆写规则 ${override.ruleId}`);
  overrideIds.add(override.ruleId);
}

console.log(
  `rulepack ok: ${pack.id}@${pack.version}, ${counts.landmine} landmine + ${counts.frustration} frustration, ` +
  `${selection.overrides.length} example overrides, schema validation passed`
);
