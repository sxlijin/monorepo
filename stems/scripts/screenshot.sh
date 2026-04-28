#!/usr/bin/env bash
set -euo pipefail

cargo build --bin multi-player
TARGET_DIR=$(cargo metadata --format-version 1 --no-deps | python3 -c "import sys, json; print(json.load(sys.stdin)['target_directory'])")
"$TARGET_DIR/debug/multi-player" "$@" &
APP_PID=$!
trap "kill $APP_PID 2>/dev/null || true" EXIT

WINDOW_ID=""
for _ in {1..100}; do
  WINDOW_ID=$("$(dirname "$0")/find_window.py" "$APP_PID" 2>/dev/null || true)
  [[ -n "$WINDOW_ID" ]] && break
  sleep 0.2
done

if [[ -z "$WINDOW_ID" ]]; then
  echo "error: window did not appear within 20s" >&2
  exit 1
fi

echo "window $WINDOW_ID found, waiting 60s before capture..."
sleep 60

mkdir -p screenshots
OUT="screenshots/$(date +%Y%m%d-%H%M%S).png"
screencapture -l "$WINDOW_ID" -o "$OUT"
echo "saved $OUT"
