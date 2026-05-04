#!/bin/sh
set -eu

ROOTFS="${1:-}"
if [ -z "${ROOTFS}" ]; then
  echo "usage: $0 <rootfs-dir>" >&2
  exit 1
fi

REPO_ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/../../.." && pwd)"
SERVO_BIN="${SERVO_BIN:-${REPO_ROOT}/third_party/servo/target/release/servoshell}"
SERVO_RUNTIME_DIR="${SERVO_RUNTIME_DIR:-}"
SOLD_BIN="${SOLD_BIN:-${REPO_ROOT}/target/release/sold}"
UI_BUILD_DIR="${UI_BUILD_DIR:-${REPO_ROOT}/ui/desktop/build}"
TARGET_ARCH="${QEMU_ARCH:-x86_64}"

if [ ! -d "${ROOTFS}" ]; then
  echo "rootfs directory not found: ${ROOTFS}" >&2
  exit 1
fi

require_linux_elf_binary() {
  bin_path="$1"
  file_info="$(file "${bin_path}")"
  case "${TARGET_ARCH}" in
    x86_64)
      case "${file_info}" in
        *"ELF 64-bit"*x86-64*) ;;
        *)
          echo "binary is not Linux x86_64 ELF: ${bin_path}" >&2
          echo "detected: ${file_info}" >&2
          echo "build Linux artifacts (not host Mach-O binaries) before staging" >&2
          exit 1
          ;;
      esac
      ;;
    aarch64|arm64)
      case "${file_info}" in
        *"ELF 64-bit"*aarch64*) ;;
        *)
          echo "binary is not Linux aarch64 ELF: ${bin_path}" >&2
          echo "detected: ${file_info}" >&2
          echo "build Linux artifacts (not host Mach-O binaries) before staging" >&2
          exit 1
          ;;
      esac
      ;;
    *)
      echo "unsupported QEMU_ARCH for binary compatibility check: ${TARGET_ARCH}" >&2
      exit 1
      ;;
  esac
}

mkdir -p "${ROOTFS}/usr/local/bin"
mkdir -p "${ROOTFS}/usr/local/share/soliloquy"
if [ ! -f "${SERVO_BIN}" ]; then
  echo "servoshell binary not found at ${SERVO_BIN}" >&2
  echo "expected in-tree fork build artifact; run ensure-servo-fork.sh first" >&2
  exit 1
fi
require_linux_elf_binary "${SERVO_BIN}"
cp "${SERVO_BIN}" "${ROOTFS}/usr/local/bin/servo"
chmod +x "${ROOTFS}/usr/local/bin/servo"

if [ -n "${SERVO_RUNTIME_DIR}" ]; then
  if [ ! -d "${SERVO_RUNTIME_DIR}" ]; then
    echo "servo runtime bundle not found: ${SERVO_RUNTIME_DIR}" >&2
    exit 1
  fi
  cp -R "${SERVO_RUNTIME_DIR}/." "${ROOTFS}/"
fi

if [ ! -f "${SOLD_BIN}" ]; then
  echo "sold binary not found at ${SOLD_BIN}" >&2
  echo "run cargo build --release -p sold before staging artifacts" >&2
  exit 1
fi
require_linux_elf_binary "${SOLD_BIN}"
cp "${SOLD_BIN}" "${ROOTFS}/usr/local/bin/sold"
chmod +x "${ROOTFS}/usr/local/bin/sold"

if [ ! -d "${UI_BUILD_DIR}" ]; then
  echo "desktop UI build not found at ${UI_BUILD_DIR}" >&2
  echo "run tools/soliloquy/build_ui.sh before staging artifacts" >&2
  exit 1
fi
rm -rf "${ROOTFS}/usr/local/share/soliloquy/ui"
cp -R "${UI_BUILD_DIR}" "${ROOTFS}/usr/local/share/soliloquy/ui"

# Stage sold bundle (includes os://terminal HTML + ghostty WASM)
BUNDLE_DIR="${REPO_ROOT}/bundle"
WASM_OUT="${BUNDLE_DIR}/terminal/ghostty-vt.wasm"
if [ ! -f "${WASM_OUT}" ]; then
  echo "ghostty-vt.wasm not found at ${WASM_OUT}; building now..."
  if command -v zig >/dev/null 2>&1; then
    "${REPO_ROOT}/scripts/build-ghostty-wasm.sh"
  else
    echo "WARNING: zig not found; os://terminal will load without WASM VT support" >&2
    echo "  Install zig 0.14+ and re-run to enable ghostty-vt.wasm" >&2
  fi
fi
cp -R "${BUNDLE_DIR}/." "${ROOTFS}/usr/local/share/soliloquy/bundle"

echo "Staged servo into ${ROOTFS}"
