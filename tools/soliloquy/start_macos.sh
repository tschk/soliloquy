#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
UI_DIR="${PROJECT_ROOT}/ui/desktop"
SERVO_BIN="${SERVO_BIN:-${PROJECT_ROOT}/third_party/servo/target/release/servoshell}"
SOL_START_URL="${SOL_START_URL:-os://terminal}"
SOL_SOLD_ORIGIN="${SOL_SOLD_ORIGIN:-http://127.0.0.1:8080}"
SOL_LAUNCH_URL="${SOL_LAUNCH_URL:-${SOL_SOLD_ORIGIN}/?url=${SOL_START_URL}}"
SOL_TOKEN="${SOL_TOKEN:-dev-token-change-me}"
SOL_BUNDLE_DIR="${SOL_BUNDLE_DIR:-${UI_DIR}/build}"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Error: start_macos.sh is for macOS."
  exit 1
fi

if ! command -v bun >/dev/null 2>&1; then
  echo "Error: bun not found."
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "Error: cargo not found."
  exit 1
fi

cd "${UI_DIR}"
if [[ ! -d node_modules ]]; then
  bun install
fi
bun run build

if [[ "${SOL_MACOS_DRY_RUN:-0}" == "1" ]]; then
  echo "sold bundle: ${SOL_BUNDLE_DIR}"
  echo "servo binary: ${SERVO_BIN}"
  echo "launch url: ${SOL_LAUNCH_URL}"
  exit 0
fi

if [[ ! -x "${SERVO_BIN}" ]]; then
  echo "Error: Servo binary not executable at ${SERVO_BIN}"
  echo "Build third_party/servo with ./mach build --release or set SERVO_BIN."
  exit 1
fi

cleanup() {
  if [[ -n "${SOLD_PID:-}" ]]; then
    kill "${SOLD_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

cd "${PROJECT_ROOT}"
SOL_TOKEN="${SOL_TOKEN}" SOL_BUNDLE_DIR="${SOL_BUNDLE_DIR}" cargo run -p sold &
SOLD_PID=$!

for _ in {1..120}; do
  if curl -fsS "${SOL_SOLD_ORIGIN}/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.25
done

if ! curl -fsS "${SOL_SOLD_ORIGIN}/health" >/dev/null 2>&1; then
  echo "Error: sold did not become ready at ${SOL_SOLD_ORIGIN}/health"
  exit 1
fi

exec "${SERVO_BIN}" --no-browser-chrome "${SOL_LAUNCH_URL}"
