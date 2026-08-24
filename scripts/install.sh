#!/usr/bin/env bash

set -euo pipefail

repo="${ALLWRIGHT_REPOSITORY:-allwright-dev/allwright}"
version="${ALLWRIGHT_VERSION:-latest}"
install_root="${ALLWRIGHT_INSTALL_DIR:-$HOME/.local/bin}"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux) os_part="unknown-linux-gnu" ;;
  Darwin) os_part="apple-darwin" ;;
  *)
    echo "unsupported OS: $os" >&2
    exit 1
    ;;
esac

case "$arch" in
  x86_64|amd64) arch_part="x86_64" ;;
  arm64|aarch64) arch_part="aarch64" ;;
  *)
    echo "unsupported architecture: $arch" >&2
    exit 1
    ;;
esac

target="${arch_part}-${os_part}"

if [[ "$version" == "latest" ]]; then
  if command -v python3 >/dev/null 2>&1; then
    version="$(python3 - <<'PY'
import json
import urllib.request

with urllib.request.urlopen("https://api.github.com/repos/allwright-dev/allwright/releases/latest") as response:
    data = json.load(response)
print(data["tag_name"])
PY
)"
  else
    echo "python3 is required when ALLWRIGHT_VERSION is not set" >&2
    exit 1
  fi
fi

asset_name="allwright-${version}-${target}.tar.gz"
download_url="https://github.com/${repo}/releases/download/${version}/${asset_name}"
archive_path="$tmp_dir/${asset_name}"

echo "Downloading ${download_url}"
curl -fL "$download_url" -o "$archive_path"

mkdir -p "$install_root"
tar -xzf "$archive_path" -C "$tmp_dir"
install -m 755 "$tmp_dir/bin/allwright" "$install_root/allwright"

echo "Installed allwright to $install_root/allwright"
case ":$PATH:" in
  *":$install_root:"*) ;;
  *)
    echo "Add $install_root to PATH to run \`allwright\` directly."
    ;;
esac
