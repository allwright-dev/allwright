#!/usr/bin/env bash

set -euo pipefail

repo="${ALLWRIGHT_REPOSITORY:-allwright-dev/allwright}"
version="${ALLWRIGHT_VERSION:-latest}"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

default_install_root() {
  local candidate
  local path_dir
  local old_ifs="$IFS"

  path_has_dir() {
    case ":$PATH:" in
      *":$1:"*) return 0 ;;
      *) return 1 ;;
    esac
  }

  can_use_dir() {
    local dir="$1"
    if [[ -d "$dir" ]]; then
      [[ -w "$dir" ]]
      return
    fi

    local parent
    parent="$(dirname "$dir")"
    [[ -d "$parent" && -w "$parent" ]]
  }

  is_tool_managed_dir() {
    case "$1" in
      *"/pnpm/"*|*"/.npm/"*|*"/.yarn/"*|*"/.volta/"*|*"/.pyenv/"*|*"/.rbenv/"*|*"/.asdf/"*|*"/.cargo/"*|*"/go/bin"*|*"/bun/bin"*)
        return 0
        ;;
      *)
        return 1
        ;;
    esac
  }

  for candidate in \
    "/usr/local/bin" \
    "/opt/homebrew/bin" \
    "$HOME/.local/bin" \
    "$HOME/bin"
  do
    if can_use_dir "$candidate"; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  IFS=":"
  for path_dir in $PATH; do
    [[ -n "$path_dir" ]] || continue
    [[ -d "$path_dir" ]] || continue
    [[ -w "$path_dir" ]] || continue
    is_tool_managed_dir "$path_dir" && continue
    printf '%s\n' "$path_dir"
    IFS="$old_ifs"
    return 0
  done
  IFS="$old_ifs"

  for candidate in "$HOME/.local/bin" "$HOME/bin"; do
    if can_use_dir "$candidate"; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  printf '%s\n' "/tmp"
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
chmod +x "$install_root/allwright"

echo "Installed allwright to $install_root/allwright"
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
