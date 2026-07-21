import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import {
  loadVerifiedCorpus,
  prepareCorpus,
  validateManifest,
} from '../prepare-real-world-corpus.mjs';

const content = Buffer.from('const corpusFixture = true;\n');
const digest = createHash('sha256').update(content).digest('hex');

const project = (overrides = {}) => ({
  name: 'fixture',
  version: '1.2.3',
  url: 'https://example.test/npm/fixture@1.2.3/fixture.js',
  sha256: digest,
  filename: 'fixture.js',
  license: 'MIT',
  ...overrides,
});

const manifest = (projects = [project()]) => ({ schemaVersion: 1, projects });

const withSandbox = async (run) => {
  const root = await mkdtemp(join(tmpdir(), 'oxc-real-world-corpus-'));
  const filesDir = join(root, 'files');
  const manifestPath = join(root, 'manifest.json');
  await mkdir(filesDir);
  await writeFile(manifestPath, JSON.stringify(manifest()));
  try {
    await run({ root, filesDir, manifestPath });
  } finally {
    await rm(root, { recursive: true, force: true });
  }
};

const response = (body = content, status = 200) => ({
  ok: status >= 200 && status < 300,
  status,
  arrayBuffer: async () => body,
});

test('validates the checked-in manifest schema boundaries', () => {
  assert.doesNotThrow(() => validateManifest(manifest()));
  assert.throws(
    () => validateManifest(manifest([project(), project({ filename: 'second.js' })])),
    /duplicate project name/,
  );
  assert.throws(
    () => validateManifest(manifest([project({ version: 'latest' })])),
    /exact version/,
  );
  assert.throws(
    () => validateManifest(manifest([project({ url: 'https:\/\/example.test\/fixture.js' })])),
    /must be HTTPS and contain/,
  );
  assert.throws(
    () => validateManifest(manifest([project({ sha256: '' })])),
    /sha256 must be a non-empty string/,
  );
  assert.throws(
    () => validateManifest(manifest([project({ filename: '../fixture.js' })])),
    /safe JavaScript basename/,
  );
  assert.throws(
    () => validateManifest({ ...manifest(), unexpected: true }),
    /unknown field/,
  );
  assert.throws(
    () => validateManifest(manifest([{ ...project(), unexpected: true }])),
    /unknown field/,
  );
});

test('downloads, verifies, and atomically publishes a missing file', async () => {
  await withSandbox(async ({ filesDir, manifestPath }) => {
    let calls = 0;
    const files = await prepareCorpus({
      filesDir,
      manifestPath,
      fetchImpl: async () => {
        calls += 1;
        return response();
      },
    });
    assert.equal(calls, 1);
    assert.equal(files.length, 1);
    assert.deepEqual(await readFile(join(filesDir, 'fixture.js')), content);
  });
});

test('verifies a cache hit without network access', async () => {
  await withSandbox(async ({ filesDir, manifestPath }) => {
    await writeFile(join(filesDir, 'fixture.js'), content);
    const files = await prepareCorpus({
      filesDir,
      manifestPath,
      fetchImpl: async () => assert.fail('cache hit must not fetch'),
    });
    assert.equal(files[0].sha256, digest);
  });
});

test('rejects a corrupt cached file instead of replacing it', async () => {
  await withSandbox(async ({ filesDir, manifestPath }) => {
    await writeFile(join(filesDir, 'fixture.js'), 'corrupt');
    await assert.rejects(
      prepareCorpus({
        filesDir,
        manifestPath,
        fetchImpl: async () => assert.fail('corrupt cache must not fetch'),
      }),
      /failed SHA-256 verification/,
    );
  });
});

test('rejects a downloaded hash mismatch without publishing a file', async () => {
  await withSandbox(async ({ filesDir, manifestPath }) => {
    await assert.rejects(
      prepareCorpus({
        filesDir,
        manifestPath,
        fetchImpl: async () => response(Buffer.from('wrong')),
      }),
      /download failed SHA-256 verification/,
    );
    await assert.rejects(readFile(join(filesDir, 'fixture.js')), /ENOENT/);
  });
});

test('propagates HTTP and transport failures', async () => {
  await withSandbox(async ({ filesDir, manifestPath }) => {
    await assert.rejects(
      prepareCorpus({ filesDir, manifestPath, fetchImpl: async () => response(content, 503) }),
      /HTTP 503/,
    );
    await assert.rejects(
      prepareCorpus({
        filesDir,
        manifestPath,
        fetchImpl: async () => {
          throw new Error('offline');
        },
      }),
      /offline/,
    );
  });
});

test('check-only mode never downloads a missing file', async () => {
  await withSandbox(async ({ filesDir, manifestPath }) => {
    await assert.rejects(
      prepareCorpus({
        filesDir,
        manifestPath,
        checkOnly: true,
        fetchImpl: async () => assert.fail('check-only must not fetch'),
      }),
      /Run 'node scripts\/prepare-real-world-corpus\.mjs'/,
    );
  });
});

test('cleans an interrupted owned temporary file before acquisition', async () => {
  await withSandbox(async ({ filesDir, manifestPath }) => {
    const interrupted = join(filesDir, '.fixture.js.download-123-stale');
    await writeFile(interrupted, 'partial');
    await prepareCorpus({ filesDir, manifestPath, fetchImpl: async () => response() });
    await assert.rejects(readFile(interrupted), /ENOENT/);
  });
});

test('rejects extra stale corpus files', async () => {
  await withSandbox(async ({ filesDir, manifestPath }) => {
    await writeFile(join(filesDir, 'fixture.js'), content);
    await writeFile(join(filesDir, 'old-version.js'), content);
    assert.throws(
      () => loadVerifiedCorpus({ filesDir, manifestPath }),
      /untracked entry "old-version\.js"/,
    );
  });
});

test('rejects non-file corpus entries', async () => {
  await withSandbox(async ({ filesDir, manifestPath }) => {
    await mkdir(join(filesDir, 'fixture.js'));
    assert.throws(
      () => loadVerifiedCorpus({ filesDir, manifestPath }),
      /must be a regular file/,
    );
  });
});

test('rejects extra directories in the corpus cache', async () => {
  await withSandbox(async ({ filesDir, manifestPath }) => {
    await writeFile(join(filesDir, 'fixture.js'), content);
    await mkdir(join(filesDir, 'stale'));
    assert.throws(
      () => loadVerifiedCorpus({ filesDir, manifestPath }),
      /untracked entry "stale"/,
    );
  });
});
