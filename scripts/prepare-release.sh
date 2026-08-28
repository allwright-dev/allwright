#!/usr/bin/env bash

set -euo pipefail

# Local maintainer helper. Run this from a clean local checkout before publishing.

usage() {
  echo "usage: $0 <semantic-version>" >&2
  echo "example: $0 0.0.8" >&2
  exit 1
}

if [[ $# -ne 1 ]]; then
  usage
fi

input_version="$1"
if [[ ! "$input_version" =~ ^v?([0-9]+)\.([0-9]+)\.([0-9]+)(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
  echo "error: expected a semantic version like 0.0.8 or v0.0.8" >&2
  exit 1
fi

version="${input_version#v}"
tag="v${version}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$repo_root"

if [[ "$(git branch --show-current)" != "main" ]]; then
  echo "error: release preparation must run from the main branch" >&2
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "error: working tree must be clean before preparing a release" >&2
  exit 1
fi

git pull --ff-only origin main

if git rev-parse --verify --quiet "refs/tags/${tag}" >/dev/null; then
  echo "error: tag ${tag} already exists locally" >&2
  exit 1
fi

if git ls-remote --tags --exit-code origin "refs/tags/${tag}" >/dev/null 2>&1; then
  echo "error: tag ${tag} already exists on origin" >&2
  exit 1
fi

bash "$repo_root/scripts/sync-version.sh" "$version"
bash "$repo_root/scripts/sync-npm-version.sh" "$version"
bash "$repo_root/scripts/sync-python-version.sh" "$version"

(cd "$repo_root/typescript/core" && bun run build)
(cd "$repo_root/typescript/vitest" && bun run build)

if [[ -n "$(git status --porcelain)" ]]; then
  git add -A
  git commit -m "chore: prepare release ${tag}"
else
  echo "versions already match ${version}; skipping release commit"
fi

git push origin main
git tag "$tag"
git push origin "$tag"

echo "prepared and pushed release ${tag}"
