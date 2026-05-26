import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { setTimeout as delay } from 'node:timers/promises';
import { instrument } from 'oxc-coverage-instrument';
import { fixtureFilename, fixtureSource } from '../src/fixture.js';

const port = await new Promise((resolve, reject) => {
  const server = createServer();
  server.on('error', reject);
  server.listen(0, '127.0.0.1', () => {
    const address = server.address();
    server.close(() => resolve(address.port));
  });
});
const url = `http://127.0.0.1:${port}/`;
const reference = JSON.parse(
  instrument(fixtureSource, fixtureFilename, { sourceMap: true }).coverageMap,
);

const child = spawn(
  process.platform === 'win32' ? 'npx.cmd' : 'npx',
  ['wrangler', 'dev', '--local', '--ip', '127.0.0.1', '--port', String(port)],
  {
    cwd: new URL('..', import.meta.url),
    env: {
      ...process.env,
      NO_COLOR: '1',
      WRANGLER_LOG: 'error',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
    detached: process.platform !== 'win32',
  },
);

let logs = '';
child.stdout.on('data', (chunk) => {
  logs += chunk.toString();
});
child.stderr.on('data', (chunk) => {
  logs += chunk.toString();
});

let childExited = false;
const childExit = new Promise((resolve) => {
  child.once('exit', (code, signal) => {
    childExited = true;
    resolve({ code, signal });
  });
});

function signalChild(signal) {
  if (childExited) return;
  try {
    if (process.platform === 'win32') {
      child.kill(signal);
    } else {
      process.kill(-child.pid, signal);
    }
  } catch (error) {
    if (error.code !== 'ESRCH') {
      throw error;
    }
  }
}

async function stopChild() {
  signalChild('SIGTERM');
  await Promise.race([childExit, delay(2000)]);
  if (!childExited) {
    signalChild('SIGKILL');
    await childExit;
  }
}

async function fetchWithRetry() {
  let lastError;
  for (let attempt = 0; attempt < 90; attempt += 1) {
    if (childExited) {
      throw lastError ?? new Error('wrangler dev exited before the Worker became reachable');
    }
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(1000) });
      const text = await response.text();
      if (response.ok) return JSON.parse(text);
      lastError = new Error(`HTTP ${response.status}: ${text}`);
    } catch (error) {
      lastError = error;
    }
    await delay(500);
  }
  throw lastError ?? new Error('Worker did not become reachable');
}

try {
  const body = await fetchWithRetry();
  assert.equal(body.ok, true, body.error ?? 'Worker response was not ok');
  assert.equal(
    body.hasSharedArrayBuffer,
    false,
    'Cloudflare Workers smoke must exercise the single-threaded browser binding',
  );
  assert.deepEqual(body.fileCoverage, reference);
  console.log('Cloudflare Workers smoke OK: single-threaded WASI matches native coverage');
} catch (error) {
  console.error(logs);
  throw error;
} finally {
  await stopChild();
}
