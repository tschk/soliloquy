#!/usr/bin/env bash
# Build libghostty-vt as a WebAssembly module for os://terminal.
#
# Output:
#   bundle/terminal/ghostty-vt.wasm   — VT state machine
#
# The wasm module is gitignored (large binary). Run this script:
#   - Manually: ./scripts/build-ghostty-wasm.sh
#   - As part of Alpine staging: system/alpine/scripts/stage-soliloquy-artifacts.sh
#     calls this automatically when the .wasm is absent.
#
# Requirements:
#   - zig 0.14+ (https://ziglang.org/download/)
#   - third_party/ghostty submodule initialised:
#       git submodule update --init third_party/ghostty
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GHOSTTY_DIR="${REPO_ROOT}/third_party/ghostty"
OUT_DIR="${REPO_ROOT}/bundle/terminal"

# ── checks ──────────────────────────────────────────────────────────────────
if ! command -v zig &>/dev/null; then
  echo "ERROR: zig not found. Install zig 0.14+ from https://ziglang.org/download/" >&2
  exit 1
fi

ZIG_VERSION="$(zig version 2>/dev/null || true)"
echo "zig: ${ZIG_VERSION}"

if [ ! -f "${GHOSTTY_DIR}/build.zig" ]; then
  echo "ERROR: ${GHOSTTY_DIR}/build.zig not found." >&2
  echo "  Run: git submodule update --init third_party/ghostty" >&2
  exit 1
fi

# ── build ────────────────────────────────────────────────────────────────────
echo "Building ghostty-vt WASM → ${OUT_DIR}/ghostty-vt.wasm"
mkdir -p "${OUT_DIR}"

cd "${GHOSTTY_DIR}"

zig build \
  -Demit-lib-vt \
  -Dtarget=wasm32-freestanding \
  -Doptimize=ReleaseSmall

# zig-out/bin/ghostty-vt.wasm is the output path documented in ghostty AGENTS.md
WASM_SRC="${GHOSTTY_DIR}/zig-out/bin/ghostty-vt.wasm"

if [ ! -f "${WASM_SRC}" ]; then
  # Fallback: some zig versions put it under lib/
  WASM_SRC="${GHOSTTY_DIR}/zig-out/lib/ghostty-vt.wasm"
fi

if [ ! -f "${WASM_SRC}" ]; then
  echo "ERROR: ghostty-vt.wasm not found after build. Check zig-out/ tree:" >&2
  find "${GHOSTTY_DIR}/zig-out" -name "*.wasm" 2>/dev/null || true
  exit 1
fi

cp "${WASM_SRC}" "${OUT_DIR}/ghostty-vt.wasm"

SIZE="$(du -sh "${OUT_DIR}/ghostty-vt.wasm" | cut -f1)"
echo "OK: ghostty-vt.wasm (${SIZE}) → ${OUT_DIR}/ghostty-vt.wasm"
