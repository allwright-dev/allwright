#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <version>" >&2
  exit 1
fi

version="${1#v}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

python3 - "$repo_root" "$version" <<'PY'
import json
import pathlib
import re
import subprocess
import sys

repo_root = pathlib.Path(sys.argv[1])
expected = sys.argv[2]

if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", expected):
    raise SystemExit(f"invalid release version: {expected}")

metadata = json.loads(
    subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=repo_root,
        text=True,
    )
)

workspace_members = set(metadata["workspace_members"])
packages = [package for package in metadata["packages"] if package["id"] in workspace_members]
rust_root = (repo_root / "rust").resolve()
release_packages = []
for package in packages:
    manifest_path = pathlib.Path(package["manifest_path"]).resolve()
    try:
        manifest_path.relative_to(rust_root)
    except ValueError:
        continue
    release_packages.append(package)

errors = []
for package in release_packages:
    if package["version"] != expected:
        errors.append(
            f'{package["name"]}: package version {package["version"]!r} != {expected!r}'
        )

    for dependency in package["dependencies"]:
        if dependency.get("path") is None or not dependency["name"].startswith("allwright"):
            continue
        requirement = dependency["req"]
        if requirement != f"^{expected}":
            errors.append(
                f'{package["name"]}: dependency {dependency["name"]} requirement '
                f'{requirement!r} != {f"^{expected}"!r}'
            )

if not release_packages:
    errors.append("no Rust release packages found")

if errors:
    raise SystemExit("Rust release version verification failed:\n- " + "\n- ".join(errors))

print(f"verified {len(release_packages)} Rust release packages at {expected}")
PY
