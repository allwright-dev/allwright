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
pyproject = repo_root / "python" / "pyproject.toml"

text = pyproject.read_text()
updated, count = re.subn(r'(?m)^version = "[^"]+"$', f'version = "{version}"', text, count=1)
if count != 1:
    raise SystemExit(f"failed to update Python package version in {pyproject}")
pyproject.write_text(updated)
PY
