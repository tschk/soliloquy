#!/bin/bash
# Full build and run Soliloquy v0 in QEMU
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

echo "Building and running Soliloquy v0 in QEMU..."

# Build rootfs
"${SCRIPT_DIR}/build-rootfs.sh"

# Ensure binaries
"${SCRIPT_DIR}/ensure-servo-fork.sh"

# Configure rootfs
"${SCRIPT_DIR}/configure-rootfs.sh"

# Stage artifacts
"${SCRIPT_DIR}/stage-soliloquy-artifacts.sh"

WORK_DIR="${PROJECT_ROOT}/build/alpine"
ROOTFS_DIR="${WORK_DIR}/rootfs"

# Create squashfs
echo "Creating squashfs image..."
mksquashfs "${ROOTFS_DIR}" "${WORK_DIR}/soliloquy.sqfs" -comp zstd

# Create disk image
echo "Creating disk image..."
qemu-img create -f qcow2 "${WORK_DIR}/soliloquy.qcow2" 2G

# Setup loop device and copy squashfs
LOOP_DEV=$(losetup -f)
losetup "${LOOP_DEV}" "${WORK_DIR}/soliloquy.qcow2"
mkfs.ext4 "${LOOP_DEV}"
mount "${LOOP_DEV}" /mnt

cp "${WORK_DIR}/soliloquy.sqfs" /mnt/
umount /mnt
losetup -d "${LOOP_DEV}"

# Run in QEMU
echo "Starting QEMU..."
qemu-system-x86_64 \
    -m 1G \
    -kernel /boot/vmlinuz-linux \
    -initrd "${WORK_DIR}/soliloquy.sqfs" \
    -append "console=ttyS0 root=/dev/sda ro init=/sbin/init" \
    -drive file="${WORK_DIR}/soliloquy.qcow2",format=qcow2 \
    -net nic -net user \
    -nographic

echo "QEMU session ended."