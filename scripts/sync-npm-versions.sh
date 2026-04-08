#!/usr/bin/env bash
# Sync npm package.json versions with the Rust workspace version.
# Usage: ./scripts/sync-npm-versions.sh <old_version> <new_version>
set -euo pipefail

VERSION="${2:-$1}"
ROOT="$(git rev-parse --show-toplevel)"

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

echo "Syncing npm versions to $VERSION..."

# Main package (with optionalDependencies)
update_optional_deps "$ROOT/napi/package.json"

# Platform packages
for dir in "$ROOT/napi/npm"/*/; do
  if [ -f "$dir/package.json" ]; then
    update_version "$dir/package.json"
  fi
done

echo "Done."
