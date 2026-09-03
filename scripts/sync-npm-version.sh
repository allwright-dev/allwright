#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <version>" >&2
  exit 1
fi

version="$1"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

node - "$repo_root" "$version" <<'JS'
const fs = require("fs");
const path = require("path");

const repoRoot = process.argv[2];
const version = process.argv[3];
const packageJsonPaths = [
  path.join(repoRoot, "typescript", "core", "package.json"),
  path.join(repoRoot, "typescript", "create", "package.json"),
  path.join(repoRoot, "typescript", "vitest", "package.json"),
];

for (const packageJsonPath of packageJsonPaths) {
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
  packageJson.version = version;
  if (packageJson.name === "@allwright.dev/vitest") {
    packageJson.dependencies = packageJson.dependencies ?? {};
    packageJson.dependencies["@allwright.dev/core"] = version;
  }
  fs.writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);
}
JS
