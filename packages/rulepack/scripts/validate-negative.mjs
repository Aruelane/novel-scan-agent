import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { Ajv2020 } from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(here, "..");

async function readJson(relativePath) {
  const absolutePath = resolve(packageRoot, relativePath);
  return JSON.parse(await readFile(absolutePath, "utf8"));
}

const [packSchema, pack, patches] = await Promise.all([
  readJson("schemas/rulepack.schema.json"),
  readJson("packs/yy-novel-bar/2026.0.0-seed.1.json"),
  readJson("examples/bad-rulepack-patches.json"),
]);

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);
const validatePack = ajv.compile(packSchema);

let passed = 0;
let failed = 0;

for (const patch of patches) {
  // Deep-clone the pack and apply patch directly to the first rule
  const mutated = JSON.parse(JSON.stringify(pack));
  const target = mutated.rules[0];

  for (const [key, value] of Object.entries(patch.patch)) {
    if (value === null) {
      // Delete the field from the target rule
      delete target[key];
    } else if (typeof value === "object" && value !== null && !Array.isArray(value)) {
      // Nested patch like { detection: { confirmationScope: "global" } }
      for (const [nestedKey, nestedValue] of Object.entries(value)) {
        if (nestedValue === null) {
          delete target[key][nestedKey];
        } else {
          target[key][nestedKey] = nestedValue;
        }
      }
    } else {
      // Direct scalar replacement on the rule
      target[key] = value;
    }
  }

  const valid = validatePack(mutated);

  if (valid) {
    failed += 1;
    console.error(`FAIL: "${patch.label}" was not rejected`);
  } else {
    // Verify the error matches expectations
    const errors = validatePack.errors ?? [];
    const matched = errors.some((e) => {
      const pathOk = e.instancePath === patch.expectedPath;
      const keywordOk = e.keyword === patch.expectedKeyword;
      let missingOk = true;
      if (patch.expectedMissing) {
        missingOk = e.params?.missingProperty === patch.expectedMissing;
      }
      return pathOk && keywordOk && missingOk;
    });

    if (matched) {
      passed += 1;
      console.log(`PASS: "${patch.label}" — ${patch.expectedPath} ${patch.expectedKeyword}${patch.expectedMissing ? " missing=" + patch.expectedMissing : ""}`);
    } else {
      failed += 1;
      console.error(`MISMATCH: "${patch.label}" was rejected but not for expected reason`);
      console.error(`  Expected: instancePath=${patch.expectedPath} keyword=${patch.expectedKeyword}${patch.expectedMissing ? " missingProperty=" + patch.expectedMissing : ""}`);
      console.error(`  Got: ${errors.map((e) => `${e.instancePath} ${e.keyword}${e.params?.missingProperty ? " missing=" + e.params.missingProperty : ""}`).join("; ")}`);
    }
  }
}

if (failed > 0) {
  console.error(`\n${failed}/${patches.length} negative tests FAILED`);
  process.exit(1);
}

console.log(`\nall ${passed} negative tests passed with correct error paths and keywords`);
