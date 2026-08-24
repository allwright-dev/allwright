#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <version>" >&2
  exit 1
fi

version="$1"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

python3 - "$repo_root" "$version" <<'PY'
import pathlib
import re
import sys

repo_root = pathlib.Path(sys.argv[1])
version = sys.argv[2]

cargo_files = [
    repo_root / "Cargo.toml",
    repo_root / "rust" / "allwright-plugin-sdk" / "Cargo.toml",
    repo_root / "rust" / "allwright-surface-web" / "Cargo.toml",
    repo_root / "rust" / "allwright-surface-mobile" / "Cargo.toml",
    repo_root / "rust" / "allwright-surface-mobile-android" / "Cargo.toml",
    repo_root / "rust" / "allwright-surface-mobile-ios" / "Cargo.toml",
    repo_root / "rust" / "allwright-surface-desktop" / "Cargo.toml",
    repo_root / "rust" / "allwright-surface-desktop-mac" / "Cargo.toml",
    repo_root / "rust" / "allwright-surface-desktop-windows" / "Cargo.toml",
    repo_root / "rust" / "allwright-surface-desktop-linux" / "Cargo.toml",
    repo_root / "rust" / "allwright" / "Cargo.toml",
    repo_root / "rust" / "allwright-cli" / "Cargo.toml",
]

workspace_pattern = re.compile(r'(?m)^version = "[^"]+"$')
dependency_pattern = re.compile(r'version = "\d+\.\d+\.\d+"')

for path in cargo_files:
    text = path.read_text()
    if path == repo_root / "Cargo.toml":
        text, count = workspace_pattern.subn(f'version = "{version}"', text, count=1)
        if count != 1:
            raise SystemExit(f"failed to update workspace version in {path}")
    else:
        text = dependency_pattern.sub(f'version = "{version}"', text)
    path.write_text(text)
PY

cargo generate-lockfile --manifest-path "$repo_root/Cargo.toml"
