import { existsSync, rmSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { dirname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const desktopDirectory = resolve(scriptDirectory, '..');
const iconSource = resolve(desktopDirectory, 'assets/app-icon.svg');
const generatedIconDirectory = resolve(desktopDirectory, 'src-tauri/icons');
const tauriEntry = resolve(
  desktopDirectory,
  'node_modules/@tauri-apps/cli/tauri.js',
);

for (const [path, name] of [
  [iconSource, 'app-icon.svg'],
  [tauriEntry, 'Tauri CLI'],
]) {
  if (!existsSync(path)) {
    throw new Error(`Missing ${name} at ${path}.`);
  }
}

await new Promise((resolveRun, rejectRun) => {
  const child = spawn(
    process.execPath,
    [tauriEntry, 'icon', iconSource, '--output', generatedIconDirectory],
    {
      cwd: desktopDirectory,
      env: process.env,
      shell: false,
      stdio: 'inherit',
    },
  );

  child.once('error', rejectRun);
  child.once('exit', (code, signal) => {
    if (signal) {
      rejectRun(new Error(`Tauri icon generation stopped by ${signal}`));
    } else if (code === 0) {
      resolveRun();
    } else {
      rejectRun(new Error(`Tauri icon generation exited with code ${code ?? 'unknown'}`));
    }
  });
});

// `tauri icon` emits every platform. This project intentionally targets only
// Windows and Android, so do not leave misleading Apple platform resources.
for (const generatedApplePath of [
  resolve(generatedIconDirectory, 'ios'),
  resolve(generatedIconDirectory, 'icon.icns'),
]) {
  if (!generatedApplePath.startsWith(`${generatedIconDirectory}${sep}`)) {
    throw new Error(`Refusing to remove path outside icon directory: ${generatedApplePath}`);
  }
  rmSync(generatedApplePath, { force: true, recursive: true });
}
