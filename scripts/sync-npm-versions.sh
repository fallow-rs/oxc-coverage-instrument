#!/usr/bin/env bash
# Sync npm package.json versions with the Rust workspace version.
# Usage: ./scripts/sync-npm-versions.sh <old_version> <new_version>
set -euo pipefail

VERSION="${2:-$1}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

update_version() {
  node -e "
    const fs = require('fs');
    const pkg = JSON.parse(fs.readFileSync('$1', 'utf8'));
    pkg.version = '$VERSION';
    fs.writeFileSync('$1', JSON.stringify(pkg, null, 2) + '\n');
  "
}

update_optional_deps() {
  node -e "
    const fs = require('fs');
    const pkg = JSON.parse(fs.readFileSync('$1', 'utf8'));
    pkg.version = '$VERSION';
    if (pkg.optionalDependencies) {
      for (const key of Object.keys(pkg.optionalDependencies)) {
        if (key.startsWith('@oxc-coverage-instrument/')) {
          pkg.optionalDependencies[key] = '$VERSION';
        }
      }
    }
    fs.writeFileSync('$1', JSON.stringify(pkg, null, 2) + '\n');
  "
}

update_lockfile() {
  node -e "
    const fs = require('fs');
    const path = '$1';
    const lock = JSON.parse(fs.readFileSync(path, 'utf8'));
    const root = lock.packages?.[''];
    if (!root) throw new Error(path + ': missing packages[\"\"]');
    lock.version = '$VERSION';
    root.version = '$VERSION';
    if (root.optionalDependencies) {
      for (const key of Object.keys(root.optionalDependencies)) {
        if (key.startsWith('@oxc-coverage-instrument/')) {
          root.optionalDependencies[key] = '$VERSION';
        }
      }
    }
    for (const [key, pkg] of Object.entries(lock.packages)) {
      const prefix = 'node_modules/@oxc-coverage-instrument/';
      if (!key.startsWith(prefix)) continue;
      const name = key.slice('node_modules/'.length);
      const basename = name.slice(name.indexOf('/') + 1);
      pkg.version = '$VERSION';
      pkg.resolved = 'https://registry.npmjs.org/' + name + '/-/' + basename + '-$VERSION.tgz';
      delete pkg.integrity;
    }
    fs.writeFileSync(path, JSON.stringify(lock, null, 2) + '\n');
  "
}

echo "Syncing npm versions to $VERSION..."

# Main package (with optionalDependencies)
update_optional_deps "$ROOT/crates/oxc-coverage-instrument-napi/package.json"
update_lockfile "$ROOT/crates/oxc-coverage-instrument-napi/package-lock.json"

# Platform packages
for dir in "$ROOT/crates/oxc-coverage-instrument-napi/npm"/*/; do
  if [ -f "$dir/package.json" ]; then
    update_version "$dir/package.json"
  fi
done

echo "Done."
