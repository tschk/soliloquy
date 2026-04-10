#!/bin/sh
set -eu

ROOTFS_DIR="${1:-build/alpine/rootfs}"
OUT_INITRD="${2:-build/alpine/qemu/rootfs.cpio.gz}"

if [ ! -d "${ROOTFS_DIR}" ]; then
  echo "rootfs directory not found: ${ROOTFS_DIR}" >&2
  exit 1
fi

mkdir -p "$(dirname "${OUT_INITRD}")"

(
  cd "${ROOTFS_DIR}"
  find . -print | cpio -o -H newc 2>/dev/null | gzip -9 > "${OUT_INITRD}"
)

echo "Built initramfs image: ${OUT_INITRD}"
