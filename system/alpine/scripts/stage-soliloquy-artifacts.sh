#!/bin/sh
set -eu

ROOTFS="${1:-}"
if [ -z "${ROOTFS}" ]; then
  echo "usage: $0 <rootfs-dir>" >&2
  exit 1
fi

REPO_ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/../../.." && pwd)"
SERVO_BIN="${SERVO_BIN:-${REPO_ROOT}/third_party/servo/target/release/servoshell}"
SOLD_BIN="${SOLD_BIN:-${REPO_ROOT}/target/release/sold}"
UI_BUILD_DIR="${UI_BUILD_DIR:-${REPO_ROOT}/ui/desktop/build}"

if [ ! -d "${ROOTFS}" ]; then
  echo "rootfs directory not found: ${ROOTFS}" >&2
  exit 1
fi

mkdir -p "${ROOTFS}/usr/local/bin"
mkdir -p "${ROOTFS}/usr/local/share/soliloquy"
if [ ! -f "${SERVO_BIN}" ]; then
  echo "servoshell binary not found at ${SERVO_BIN}" >&2
  echo "expected in-tree fork build artifact; run ensure-servo-fork.sh first" >&2
  exit 1
fi
cp "${SERVO_BIN}" "${ROOTFS}/usr/local/bin/servo"
chmod +x "${ROOTFS}/usr/local/bin/servo"

if [ ! -f "${SOLD_BIN}" ]; then
  echo "sold binary not found at ${SOLD_BIN}" >&2
  echo "run cargo build --release -p sold before staging artifacts" >&2
  exit 1
fi
cp "${SOLD_BIN}" "${ROOTFS}/usr/local/bin/sold"
chmod +x "${ROOTFS}/usr/local/bin/sold"

if [ ! -d "${UI_BUILD_DIR}" ]; then
  echo "desktop UI build not found at ${UI_BUILD_DIR}" >&2
  echo "run tools/soliloquy/build_ui.sh before staging artifacts" >&2
  exit 1
fi
rm -rf "${ROOTFS}/usr/local/share/soliloquy/ui"
cp -R "${UI_BUILD_DIR}" "${ROOTFS}/usr/local/share/soliloquy/ui"
echo "Staged servo into ${ROOTFS}"
