import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const capPath = resolve(scriptDir, '..', 'src-tauri', 'capabilities', 'main-local.json');

let cap;
try {
  cap = JSON.parse(readFileSync(capPath, 'utf-8'));
} catch (err) {
  console.error('CAPABILITY VALIDATE FAIL');
  console.error(`reason: could not read capability file: ${err.message}`);
  process.exit(1);
}

const permissions = cap.permissions ?? [];

// Assert permissions contain only allowed permissions
const allowed = new Set(['allow-import-capabilities', 'allow-rule-pack-summary']);
for (const perm of permissions) {
  if (!allowed.has(perm)) {
    console.error('CAPABILITY VALIDATE FAIL');
    console.error(`reason: unexpected permission "${perm}"`);
    console.error(`allowed: ${JSON.stringify([...allowed])}`);
    process.exit(1);
  }
}
if (permissions.length === 0) {
  console.error('CAPABILITY VALIDATE FAIL');
  console.error('reason: permissions array is empty');
  process.exit(1);
}

// Assert forbidden permissions are NOT present
const forbidden = ['sql:default', 'sql:allow-execute', 'dialog:allow-open'];
for (const perm of forbidden) {
  if (permissions.includes(perm)) {
    console.error('CAPABILITY VALIDATE FAIL');
    console.error(`reason: forbidden permission "${perm}" found in WebView capability`);
    process.exit(1);
  }
}

console.log('OK: Capability permissions are valid.');
process.exit(0);
