#!/usr/bin/env bash

set -euo pipefail

if [[ "$#" -ne 2 ]]; then
  echo "usage: $0 <tag> <asset-path>" >&2
  exit 2
fi

tag="$1"
asset_path="$2"
repository="${GITHUB_REPOSITORY:-allwright-dev/allwright}"
attempts="${ALLWRIGHT_RELEASE_UPLOAD_ATTEMPTS:-5}"

if [[ ! -f "$asset_path" ]]; then
  echo "release asset does not exist: $asset_path" >&2
  exit 1
fi

error_log="$(mktemp)"
trap 'rm -f "$error_log"' EXIT

for ((attempt = 1; attempt <= attempts; attempt++)); do
  if gh release upload "$tag" "$asset_path" --clobber --repo "$repository" 2>"$error_log"; then
    echo "Uploaded $(basename "$asset_path") to ${repository}@${tag}"
    exit 0
  fi

  echo "::warning::GitHub release upload attempt ${attempt}/${attempts} failed for $(basename "$asset_path")"
  head -c 2000 "$error_log" >&2 || true
  echo >&2

  if [[ "$attempt" -lt "$attempts" ]]; then
    sleep_seconds=$((attempt * 15))
    echo "Retrying in ${sleep_seconds}s..."
    sleep "$sleep_seconds"
  fi
done

echo "failed to upload $(basename "$asset_path") after ${attempts} attempts" >&2
exit 1
