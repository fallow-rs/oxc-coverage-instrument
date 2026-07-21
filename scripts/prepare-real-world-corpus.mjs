#!/usr/bin/env node

import { createHash, randomUUID } from 'node:crypto';
import {
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, '..');

export const defaultManifestPath = join(scriptDir, 'real-world-corpus.json');
export const defaultFilesDir = join(rootDir, '.bench-tmp', 'files');

const manifestKeys = new Set(['schemaVersion', 'projects']);
const projectKeys = new Set(['name', 'version', 'url', 'sha256', 'filename', 'license']);
const exactVersionPattern = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const digestPattern = /^[0-9a-f]{64}$/;
const namePattern = /^[a-z0-9][a-z0-9-]*$/;
const licensePattern = /^[A-Za-z0-9-.+]+$/;

const assertPlainObject = (value, label) => {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
};

const rejectUnknownKeys = (value, allowed, label) => {
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) {
      throw new Error(`${label} contains unknown field ${JSON.stringify(key)}`);
    }
  }
};

export const validateManifest = (manifest) => {
  assertPlainObject(manifest, 'corpus manifest');
  rejectUnknownKeys(manifest, manifestKeys, 'corpus manifest');
  if (manifest.schemaVersion !== 1) {
    throw new Error('corpus manifest schemaVersion must be 1');
  }
  if (!Array.isArray(manifest.projects) || manifest.projects.length === 0) {
    throw new Error('corpus manifest projects must be a non-empty array');
  }

  const names = new Set();
  const filenames = new Set();
  for (const [index, project] of manifest.projects.entries()) {
    const label = `corpus project at index ${index}`;
    assertPlainObject(project, label);
    rejectUnknownKeys(project, projectKeys, label);
    for (const key of projectKeys) {
      if (typeof project[key] !== 'string' || project[key].length === 0) {
        throw new Error(`${label}.${key} must be a non-empty string`);
      }
    }
    if (!namePattern.test(project.name)) {
      throw new Error(`${label}.name must be a lowercase package label`);
    }
    if (names.has(project.name)) {
      throw new Error(`corpus manifest contains duplicate project name ${project.name}`);
    }
    names.add(project.name);
    if (!exactVersionPattern.test(project.version)) {
      throw new Error(`${project.name}.version must be an exact version`);
    }
    let url;
    try {
      url = new URL(project.url);
    } catch {
      throw new Error(`${project.name}.url must be a valid URL`);
    }
    if (url.protocol !== 'https:' || !project.url.includes(`@${project.version}/`)) {
      throw new Error(`${project.name}.url must be HTTPS and contain @${project.version}/`);
    }
    if (!digestPattern.test(project.sha256)) {
      throw new Error(`${project.name}.sha256 must be 64 lowercase hex characters`);
    }
    if (
      basename(project.filename) !== project.filename ||
      project.filename === '.' ||
      project.filename === '..' ||
      project.filename.includes('\\') ||
      !project.filename.endsWith('.js')
    ) {
      throw new Error(`${project.name}.filename must be a safe JavaScript basename`);
    }
    if (filenames.has(project.filename)) {
      throw new Error(`corpus manifest contains duplicate filename ${project.filename}`);
    }
    filenames.add(project.filename);
    if (!licensePattern.test(project.license)) {
      throw new Error(`${project.name}.license must be an SPDX license identifier`);
    }
  }
  return manifest;
};

export const loadCorpusManifest = (manifestPath = defaultManifestPath) => {
  let manifest;
  try {
    manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  } catch (error) {
    throw new Error(`cannot read corpus manifest ${manifestPath}: ${error.message}`);
  }
  return validateManifest(manifest);
};

export const sha256 = (content) => createHash('sha256').update(content).digest('hex');

const verifyProjectFile = (project, filesDir) => {
  const path = join(filesDir, project.filename);
  if (!existsSync(path)) {
    throw new Error(
      `${project.name}@${project.version} is missing at ${path}. ` +
        "Run 'node scripts/prepare-real-world-corpus.mjs'.",
    );
  }
  if (!lstatSync(path).isFile()) {
    throw new Error(`${project.name}@${project.version} must be a regular file at ${path}`);
  }
  const actual = sha256(readFileSync(path));
  if (actual !== project.sha256) {
    throw new Error(
      `${project.name}@${project.version} failed SHA-256 verification at ${path}: ` +
        `expected ${project.sha256}, got ${actual}`,
    );
  }
  return { ...project, path };
};

const unexpectedFiles = (projects, filesDir) => {
  if (!existsSync(filesDir)) return [];
  const expected = new Set(projects.map((project) => project.filename));
  return readdirSync(filesDir, { withFileTypes: true })
    .filter((entry) => !expected.has(entry.name))
    .map((entry) => entry.name)
    .sort();
};

export const loadVerifiedCorpus = ({
  manifestPath = defaultManifestPath,
  filesDir = defaultFilesDir,
  limit,
  rejectExtras = true,
} = {}) => {
  const manifest = loadCorpusManifest(manifestPath);
  const projects = limit === undefined ? manifest.projects : manifest.projects.slice(0, limit);
  if (rejectExtras) {
    const extras = unexpectedFiles(manifest.projects, filesDir);
    if (extras.length > 0) {
      throw new Error(
        `real-world corpus contains untracked entry ${JSON.stringify(extras[0])} under ${filesDir}`,
      );
    }
  }
  return projects.map((project) => verifyProjectFile(project, filesDir));
};

const cleanupInterruptedDownloads = (projects, filesDir) => {
  if (!existsSync(filesDir)) return;
  const prefixes = projects.map((project) => `.${project.filename}.download-`);
  for (const entry of readdirSync(filesDir, { withFileTypes: true })) {
    if (entry.isFile() && prefixes.some((prefix) => entry.name.startsWith(prefix))) {
      rmSync(join(filesDir, entry.name));
    }
  }
};

const downloadProject = async (project, filesDir, fetchImpl) => {
  const target = join(filesDir, project.filename);
  const temporary = join(filesDir, `.${project.filename}.download-${process.pid}-${randomUUID()}`);
  let response;
  try {
    response = await fetchImpl(project.url);
  } catch (error) {
    throw new Error(`failed to download ${project.name}@${project.version}: ${error.message}`);
  }
  if (!response.ok) {
    throw new Error(
      `failed to download ${project.name}@${project.version}: HTTP ${response.status}`,
    );
  }
  const content = Buffer.from(await response.arrayBuffer());
  const actual = sha256(content);
  if (actual !== project.sha256) {
    throw new Error(
      `${project.name}@${project.version} download failed SHA-256 verification: ` +
        `expected ${project.sha256}, got ${actual}`,
    );
  }
  try {
    writeFileSync(temporary, content, { flag: 'wx' });
    renameSync(temporary, target);
  } finally {
    rmSync(temporary, { force: true });
  }
};

export const prepareCorpus = async ({
  manifestPath = defaultManifestPath,
  filesDir = defaultFilesDir,
  checkOnly = false,
  limit,
  fetchImpl = globalThis.fetch,
} = {}) => {
  const manifest = loadCorpusManifest(manifestPath);
  const projects = limit === undefined ? manifest.projects : manifest.projects.slice(0, limit);
  if (checkOnly) {
    return loadVerifiedCorpus({ manifestPath, filesDir, limit });
  }

  mkdirSync(filesDir, { recursive: true });
  cleanupInterruptedDownloads(manifest.projects, filesDir);
  for (const project of projects) {
    const target = join(filesDir, project.filename);
    if (existsSync(target)) {
      verifyProjectFile(project, filesDir);
      continue;
    }
    process.stderr.write(`Downloading ${project.name}@${project.version}...\n`);
    await downloadProject(project, filesDir, fetchImpl);
  }
  return loadVerifiedCorpus({ manifestPath, filesDir, limit });
};

const parseArguments = (args) => {
  const options = { checkOnly: false, printPaths: false, limit: undefined };
  for (const arg of args) {
    if (arg === '--check-only') {
      options.checkOnly = true;
    } else if (arg === '--print-paths') {
      options.printPaths = true;
    } else if (arg.startsWith('--limit=')) {
      const limit = Number.parseInt(arg.slice('--limit='.length), 10);
      if (!Number.isSafeInteger(limit) || limit < 1) {
        throw new Error('--limit must be a positive integer');
      }
      options.limit = limit;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  return options;
};

const main = async () => {
  const options = parseArguments(process.argv.slice(2));
  const projects = await prepareCorpus(options);
  if (options.printPaths) {
    for (const project of projects) process.stdout.write(`${project.path}\n`);
  } else {
    process.stderr.write(`Verified ${projects.length} pinned real-world corpus files.\n`);
  }
};

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`real-world corpus: ${error.message}`);
    process.exitCode = 1;
  });
}
