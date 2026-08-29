#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  cat <<'EOF'
Usage:
  ./scripts/source-mobile-android.sh /path/to/app.apk [extra args...]

Examples:
  ./scripts/source-mobile-android.sh ./app-debug.apk --app-id dev.allwright.sample
  ./scripts/source-mobile-android.sh ./app-debug.apk --out tmp/flights-source.xml --device 'QA Galaxy S24'
EOF
  exit 1
fi

apk_path=$1
shift

if [[ -x ".venv-mobile-android/bin/python" && -z "${ALLWRIGHT_ANDROID_PYTHON:-}" ]]; then
  export ALLWRIGHT_ANDROID_PYTHON="$PWD/.venv-mobile-android/bin/python"
fi

mkdir -p tmp

cargo run -p allwright-surface-mobile-android --example source -- \
  --apk "$apk_path" \
  "$@"
