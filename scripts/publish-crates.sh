#!/usr/bin/env bash

set -euo pipefail

web_crates=(
  allwright-plugin-sdk
  allwright-surface-web
  allwright-core
  allwright
)

full_crates=(
  allwright-plugin-sdk
  allwright-surface-mobile
  allwright-surface-desktop
  allwright-surface-web
  allwright-surface-mobile-android
  allwright-surface-mobile-ios
  allwright-surface-desktop-mac
  allwright-surface-desktop-windows
  allwright-surface-desktop-linux
  allwright-core
  allwright
)

mode="${1:-publish}"
profile="${2:-web}"
publish_interval_seconds="${PUBLISH_INTERVAL_SECONDS:-20}"

if [[ "$mode" != "publish" && "$mode" != "dry-run" ]]; then
  echo "usage: $0 [publish|dry-run] [web|full]" >&2
  exit 1
fi

if [[ "$profile" != "web" && "$profile" != "full" ]]; then
  echo "usage: $0 [publish|dry-run] [web|full]" >&2
  exit 1
fi

if [[ "$profile" == "web" ]]; then
  crates=("${web_crates[@]}")
else
  crates=("${full_crates[@]}")
fi

publish_args=(publish)
if [[ "$mode" == "dry-run" ]]; then
  publish_args+=(--dry-run)
fi

retry_after_epoch() {
  local output_file="$1"

  python3 - "$output_file" <<'PY'
import email.utils
import pathlib
import re
import sys
from datetime import datetime, timezone

text = pathlib.Path(sys.argv[1]).read_text()
match = re.search(r"Please try again after (.+?) and see", text)
if not match:
    sys.exit(1)

raw = match.group(1).strip()
dt = email.utils.parsedate_to_datetime(raw)
if dt is None:
    sys.exit(1)
if dt.tzinfo is None:
    dt = dt.replace(tzinfo=timezone.utc)
print(int(dt.timestamp()))
PY
}

sleep_until_retry_window() {
  local output_file="$1"
  local retry_epoch
  retry_epoch="$(retry_after_epoch "$output_file")" || {
    echo "publish failed and no crates.io retry timestamp was found" >&2
    return 1
  }

  local now_epoch
  now_epoch="$(date -u +%s)"
  local sleep_seconds=$((retry_epoch - now_epoch + 5))
  if (( sleep_seconds < 5 )); then
    sleep_seconds=5
  fi

  echo "crates.io rate limit hit; sleeping ${sleep_seconds}s until retry window opens" >&2
  sleep "$sleep_seconds"
}

publish_one() {
  local crate="$1"
  local tmp
  tmp="$(mktemp)"

  while true; do
    echo "==> cargo ${publish_args[*]} -p ${crate}"
    if cargo "${publish_args[@]}" -p "$crate" 2>&1 | tee "$tmp"; then
      rm -f "$tmp"
      return 0
    fi

    if grep -q "already exists on crates.io index" "$tmp"; then
      echo "crate ${crate} is already published at this version; skipping"
      rm -f "$tmp"
      return 0
    fi

    if grep -q "429 Too Many Requests" "$tmp"; then
      sleep_until_retry_window "$tmp"
      continue
    fi

    echo "publish failed for ${crate}" >&2
    rm -f "$tmp"
    return 1
  done
}

for i in "${!crates[@]}"; do
  crate="${crates[$i]}"
  publish_one "$crate"

  if [[ "$mode" == "publish" && "$i" -lt $((${#crates[@]} - 1)) ]]; then
    echo "waiting ${publish_interval_seconds}s before the next publish attempt"
    sleep "$publish_interval_seconds"
  fi
done
