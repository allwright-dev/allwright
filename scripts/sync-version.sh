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
java_build_file = repo_root / "java" / "build.gradle.kts"
version_source_files = [
    repo_root / "go" / "bootstrap.go",
    repo_root / "java" / "src" / "main" / "java" / "dev" / "allwright" / "client" / "BootstrapSupport.java",
    repo_root / "python" / "allwright" / "_bootstrap.py",
    repo_root / "typescript" / "core" / "src" / "bootstrap.ts",
]

workspace_pattern = re.compile(r'(?m)^version = "[^"]+"$')
internal_dependency_pattern = re.compile(
    r'(?m)^(?P<prefix>\s*allwright(?:-[A-Za-z0-9-]+)?\s*=\s*\{[^\n]*\bversion\s*=\s*")[^"]+(?P<suffix>")'
)
java_version_pattern = re.compile(r'\.orElse\("\d+\.\d+\.\d+"\)')
release_version_pattern = re.compile(r'(?m)^(?P<prefix>\s*(?:private\s+static\s+final\s+String|const|DEFAULT_RELEASE_VERSION)\s+\w*\s*=?\s*"?DEFAULT_RELEASE_VERSION"?\s*(?:=|:)\s*"?)\d+\.\d+\.\d+(?P<suffix>"[;]?)$')

source_specific_patterns = {
    repo_root / "go" / "bootstrap.go": re.compile(
        r'(?m)^(?P<prefix>\s*defaultReleaseVersion\s*=\s*")\d+\.\d+\.\d+(?P<suffix>")$'
    ),
    repo_root / "java" / "src" / "main" / "java" / "dev" / "allwright" / "client" / "BootstrapSupport.java": re.compile(
        r'(?m)^(?P<prefix>\s*private static final String DEFAULT_RELEASE_VERSION = ")\d+\.\d+\.\d+(?P<suffix>";)$'
    ),
    repo_root / "python" / "allwright" / "_bootstrap.py": re.compile(
        r'(?m)^(?P<prefix>DEFAULT_RELEASE_VERSION = ")\d+\.\d+\.\d+(?P<suffix>")$'
    ),
    repo_root / "typescript" / "core" / "src" / "bootstrap.ts": re.compile(
        r'(?m)^(?P<prefix>const DEFAULT_RELEASE_VERSION = ")\d+\.\d+\.\d+(?P<suffix>";)$'
    ),
}

for path in cargo_files:
    text = path.read_text()
    if path == repo_root / "Cargo.toml":
        text, count = workspace_pattern.subn(f'version = "{version}"', text, count=1)
        if count != 1:
            raise SystemExit(f"failed to update workspace version in {path}")
    else:
        text = internal_dependency_pattern.sub(rf'\g<prefix>{version}\g<suffix>', text)
    path.write_text(text)

java_text = java_build_file.read_text()
java_text, count = java_version_pattern.subn(f'.orElse("{version}")', java_text, count=1)
if count != 1:
    raise SystemExit(f"failed to update Java version fallback in {java_build_file}")
java_build_file.write_text(java_text)

for path in version_source_files:
    text = path.read_text()
    pattern = source_specific_patterns[path]
    updated, count = pattern.subn(rf'\g<prefix>{version}\g<suffix>', text, count=1)
    if count != 1:
        raise SystemExit(f"failed to update default release version in {path}")
    path.write_text(updated)
PY

cargo generate-lockfile --manifest-path "$repo_root/Cargo.toml"
