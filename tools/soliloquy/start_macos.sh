#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
SERVO_BIN="${SERVO_BIN:-${PROJECT_ROOT}/third_party/servo/target/release/servoshell}"
SOL_START_URL="${SOL_START_URL:-https://example.com}"
SOL_WINDOW_SIZE="${SOL_WINDOW_SIZE:-1280x820}"
SOL_DESKTOP_CHROME="${SOL_DESKTOP_CHROME:-crepuscularity}"
SOL_SOLD_ORIGIN="${SOL_SOLD_ORIGIN:-http://127.0.0.1:8080}"
SOLD_PID=""

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Error: start_macos.sh is for macOS."
  exit 1
fi

if [[ ! -x "${SERVO_BIN}" ]]; then
  echo "Error: Servo binary not executable at ${SERVO_BIN}"
  echo "Build third_party/servo with cargo build --release --manifest-path third_party/servo/ports/servoshell/Cargo.toml or set SERVO_BIN."
  exit 1
fi

if ! "${SERVO_BIN}" --help 2>&1 | grep -q -- '--no-browser-chrome'; then
  echo "Error: Servo binary does not support --no-browser-chrome: ${SERVO_BIN}"
  echo "Rebuild the local Servo fork before launching the desktop browser."
  exit 1
fi

if [[ "${SOL_MACOS_DRY_RUN:-0}" == "1" ]]; then
  echo "servo binary: ${SERVO_BIN}"
  echo "desktop chrome: ${SOL_DESKTOP_CHROME}"
  echo "desktop browser url: ${SOL_START_URL}"
  echo "window size: ${SOL_WINDOW_SIZE}"
  echo "sold origin: ${SOL_SOLD_ORIGIN}"
  exit 0
fi

cd "${PROJECT_ROOT}"

sold_ready() {
  curl -fsS "${SOL_SOLD_ORIGIN}/api/device" >/dev/null 2>&1
}

cleanup() {
  if [[ -n "${SOLD_PID}" ]]; then
    kill "${SOLD_PID}" >/dev/null 2>&1 || true
    wait "${SOLD_PID}" >/dev/null 2>&1 || true
  fi
}

ensure_sold() {
  if sold_ready; then
    return 0
  fi

  cargo run -p sold &
  SOLD_PID="$!"
  trap cleanup EXIT INT TERM

  for _ in {1..30}; do
    if sold_ready; then
      return 0
    fi
    sleep 1
  done

  echo "Error: sold did not become ready at ${SOL_SOLD_ORIGIN}"
  exit 1
}

ensure_sold

if [[ "${SOL_DESKTOP_CHROME}" == "crepuscularity" ]]; then
  env SERVO_BIN="${SERVO_BIN}" \
    SOL_START_URL="${SOL_START_URL}" \
    SOL_WINDOW_SIZE="${SOL_WINDOW_SIZE}" \
    SOL_SOLD_ORIGIN="${SOL_SOLD_ORIGIN}" \
    SOLILOQUY_JS_ENGINE="${SOLILOQUY_JS_ENGINE:-v8-experimental}" \
    cargo run -p soliloquy-shell --bin soliloquy_desktop --no-default-features --features "desktop gpui"
  exit $?
fi

env SOLILOQUY_JS_ENGINE="${SOLILOQUY_JS_ENGINE:-v8-experimental}" \
  "${SERVO_BIN}" --no-browser-chrome --window-size "${SOL_WINDOW_SIZE}" "${SOL_START_URL}"
