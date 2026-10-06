#!/usr/bin/env bash

set -euo pipefail

repo="${ALLWRIGHT_REPOSITORY:-allwright-dev/allwright}"
version="${ALLWRIGHT_VERSION:-latest}"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

# Fixed, sudo-free default: ~/.local/bin on both Linux and macOS.
default_install_root() {
  printf '%s\n' "$HOME/.local/bin"
}

install_root="${ALLWRIGHT_INSTALL_DIR:-$(default_install_root)}"

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
    version="$(python3 - "$repo" <<'PY'
import json
import sys
import urllib.request

repository = sys.argv[1]
with urllib.request.urlopen(f"https://api.github.com/repos/{repository}/releases/latest") as response:
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
chmod +x "$install_root/allwright"

echo "Installed allwright to $install_root/allwright"
installed_version="$("$install_root/allwright" --version)"
expected_version="${version#v}"
if [[ "$installed_version" != "allwright $expected_version" ]]; then
  echo "error: installed binary reports '$installed_version'; expected 'allwright $expected_version'" >&2
  exit 1
fi

# Remove copies left by earlier installer versions. Presence is checked first
# (no privileges needed); sudo is only used when a stale copy exists in a
# directory we cannot write to.
remove_legacy_installs() {
  local dir legacy
  for dir in "/usr/local/bin" "/opt/homebrew/bin" "$HOME/bin"; do
    legacy="$dir/allwright"
    [[ "$dir" == "$install_root" ]] && continue
    [[ -f "$legacy" || -L "$legacy" ]] || continue

    echo "Removing previously installed $legacy"
    if [[ -w "$dir" ]]; then
      rm -f "$legacy" || echo "warning: could not remove $legacy" >&2
    elif command -v sudo >/dev/null 2>&1; then
      echo "Administrator access is required to remove $legacy"
      sudo rm -f "$legacy" || echo "warning: could not remove $legacy; remove it manually" >&2
    else
      echo "warning: $legacy is not writable and sudo is unavailable; remove it manually" >&2
    fi
  done
}

remove_legacy_installs
hash -r 2>/dev/null || true

resolved_allwright="$(command -v allwright 2>/dev/null || true)"
if [[ -n "$resolved_allwright" && "$resolved_allwright" != "$install_root/allwright" ]]; then
  echo >&2
  echo "warning: your shell still resolves allwright to $resolved_allwright" >&2
  echo "The newly installed $install_root/allwright may be shadowed by an older binary." >&2
  echo "Move $install_root earlier on PATH or remove the stale executable, then run: hash -r" >&2
fi
case ":$PATH:" in
  *":$install_root:"*) ;;
  *)
    echo
    echo "This install directory is not on PATH in the current shell."
    echo "Run this in your shell before using \`allwright\`:"
    echo "  export PATH=\"$install_root:\$PATH\""
    echo "  hash -r"
    echo
    echo "To make it permanent, add this to your shell profile:"
    echo "  export PATH=\"$install_root:\$PATH\""
    ;;
esac

echo "Future releases can be installed with: allwright update"
