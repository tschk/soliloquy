#!/bin/sh
set -eu

QEMU_DIR="${1:-build/alpine/qemu}"
KERNEL="${QEMU_DIR}/vmlinuz-virt"
INITRAMFS="${QEMU_DIR}/rootfs.cpio.gz"
QEMU_HEADLESS="${QEMU_HEADLESS:-0}"
QEMU_ACCEL="${QEMU_ACCEL:-tcg}"
KERNEL_CMDLINE="${KERNEL_CMDLINE:-console=tty0 console=ttyS0 rdinit=/init}"

for bin in qemu-system-x86_64; do
  command -v "${bin}" >/dev/null 2>&1 || {
    echo "missing required tool: ${bin}" >&2
    exit 1
  }
done

if [ ! -f "${KERNEL}" ] || [ ! -f "${INITRAMFS}" ]; then
  echo "missing kernel/initramfs in ${QEMU_DIR} (run fetch-qemu-kernel.sh + build-qemu-initramfs.sh)" >&2
  exit 1
fi

DISPLAY_FLAGS="-display default"
if [ "${QEMU_HEADLESS}" = "1" ]; then
  DISPLAY_FLAGS="-nographic"
elif [ "$(uname -s)" = "Darwin" ]; then
  DISPLAY_FLAGS="-display cocoa"
fi

qemu-system-x86_64 \
  -machine q35,accel="${QEMU_ACCEL}" \
  -m 1536 \
  -smp 2 \
  -serial mon:stdio \
  ${DISPLAY_FLAGS} \
  -vga virtio \
  -device virtio-keyboard-pci \
  -device virtio-mouse-pci \
  -device virtio-net-pci,netdev=n1 \
  -netdev user,id=n1 \
  -kernel "${KERNEL}" \
  -initrd "${INITRAMFS}" \
  -append "${KERNEL_CMDLINE}"
