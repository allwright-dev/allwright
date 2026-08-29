#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  cat <<'EOF'
Usage:
  ./scripts/test-mobile-android.sh /path/to/app.apk 'xpath=//*[@text="Login"]' [extra args...]

Examples:
  ./scripts/test-mobile-android.sh ./app-debug.apk 'xpath=//*[@text="Get Started"]' --app-id dev.allwright.sample
  ./scripts/test-mobile-android.sh ./app-debug.apk 'css=#com.example:id/login' --device 'QA Galaxy S24'
EOF
  exit 1
fi

apk_path=$1
selector=$2
shift 2

if [[ -x ".venv-mobile-android/bin/python" && -z "${ALLWRIGHT_ANDROID_PYTHON:-}" ]]; then
  export ALLWRIGHT_ANDROID_PYTHON="$PWD/.venv-mobile-android/bin/python"
fi

cargo run -p allwright-surface-mobile-android --example smoke -- \
  --apk "$apk_path" \
  --selector "$selector" \
  "$@"
