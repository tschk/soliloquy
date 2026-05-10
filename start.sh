#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
MODE="qemu"

usage() {
  cat <<'USAGE'
Usage: ./start.sh [qemu|dev] [options]

Modes:
  qemu         Build/stage Soliloquy appliance and boot QEMU (default)
  dev          Run scripts/dev.sh for local development

Options:
  --build-only Prepare QEMU artifacts, skip VM launch (same as QEMU_RUN=0)
  --headless   Run QEMU in headless serial mode
  -h, --help   Show this help

Environment (passed through to system/alpine/scripts/qemu-v0.sh):
  QEMU_ARCH=x86_64
  SERVO_FORCE_REBUILD=1
  FORCE_ROOTFS_REBUILD=1   full Docker rootfs when packages/overlay scripts change
  SOL_START_URL=os://terminal
  SOL_LAUNCH_URL=http://127.0.0.1:8080/terminal   override first Servo URL
  SOL_SERVO_VERBOSE=1   noisy servoshell / wget / env logging
  SOL_SESSION_X11_FALLBACK=0   disable XWayland retry after a failed Wayland session
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    qemu|dev)
      MODE="$1"
      ;;
    --build-only)
      export QEMU_RUN=0
      ;;
    --headless)
      export QEMU_HEADLESS=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "start.sh: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

case "${MODE}" in
  qemu)
    exec "${ROOT_DIR}/system/alpine/scripts/qemu-v0.sh"
    ;;
  dev)
    exec "${ROOT_DIR}/scripts/dev.sh"
    ;;
esac
