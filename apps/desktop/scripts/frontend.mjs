import { existsSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const desktopDirectory = resolve(scriptDirectory, '..');
const clientDirectory = resolve(desktopDirectory, '../client');
const viteEntry = resolve(clientDirectory, 'node_modules/vite/bin/vite.js');
const typescriptEntry = resolve(clientDirectory, 'node_modules/typescript/bin/tsc');

function requireEntry(entry, packageName) {
  if (!existsSync(entry)) {
    throw new Error(
      `Missing ${packageName} at ${entry}. Run npm install in apps/client first.`,
    );
  }
}

function runNode(entry, args) {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(process.execPath, [entry, ...args], {
      cwd: clientDirectory,
      env: process.env,
      shell: false,
      stdio: 'inherit',
    });

    child.once('error', rejectRun);
    child.once('exit', (code, signal) => {
      if (signal) {
        rejectRun(new Error(`${entry} stopped by ${signal}`));
      } else if (code === 0) {
        resolveRun();
      } else {
        rejectRun(new Error(`${entry} exited with code ${code ?? 'unknown'}`));
      }
    });
  });
}

const action = process.argv[2];

if (action === 'dev') {
  requireEntry(viteEntry, 'Vite');
  const host = process.env.TAURI_DEV_HOST || '127.0.0.1';
  await runNode(viteEntry, ['--host', host, '--strictPort']);
} else if (action === 'build') {
  requireEntry(typescriptEntry, 'TypeScript');
  requireEntry(viteEntry, 'Vite');
  await runNode(typescriptEntry, ['-b']);
  await runNode(viteEntry, ['build']);
} else {
  throw new Error('Expected `dev` or `build`.');
}
