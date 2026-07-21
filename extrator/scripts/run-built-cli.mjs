#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const cliPath = resolve(scriptDirectory, '../dist/cli.js');

if (!existsSync(cliPath)) {
  console.error(`[extrator] missing build artifact: ${cliPath}`);
  console.error('[extrator] run manually: env -C extrator npm run build');
  process.exit(1);
}

const result = spawnSync(process.execPath, [cliPath, ...process.argv.slice(2)], {
  stdio: 'inherit',
});

if (result.error) {
  throw result.error;
}

if (result.signal === 'SIGINT') {
  process.exit(130);
}

if (result.signal) {
  console.error(`[extrator] CLI terminated with signal ${result.signal}`);
  process.exit(1);
}

process.exit(result.status ?? 0);
