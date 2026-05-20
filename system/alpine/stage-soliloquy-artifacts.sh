#!/bin/bash
# Stage Soliloquy artifacts into rootfs
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
WORK_DIR="${PROJECT_ROOT}/build/alpine"
ROOTFS_DIR="${WORK_DIR}/rootfs"

echo "Staging Soliloquy artifacts..."

cd "${PROJECT_ROOT}"

# Copy binaries
echo "Copying binaries..."
cp "target/x86_64-unknown-linux-musl/release/soliloquy_shell" "${ROOTFS_DIR}/usr/local/bin/"
cp "target/x86_64-unknown-linux-musl/release/sold" "${ROOTFS_DIR}/usr/local/bin/"
cp "${SCRIPT_DIR}/scripts/sol-session-start" "${ROOTFS_DIR}/usr/local/bin/sol-session-start"
cp "${SCRIPT_DIR}/scripts/sol-servo-wrapper" "${ROOTFS_DIR}/usr/local/bin/sol-servo-wrapper"
chmod +x "${ROOTFS_DIR}/usr/local/bin/sol-session-start" "${ROOTFS_DIR}/usr/local/bin/sol-servo-wrapper"

WAX_BIN="${PROJECT_ROOT}/../wax/build/wax"
if [ -x "${WAX_BIN}" ]; then
    cp "${WAX_BIN}" "${ROOTFS_DIR}/usr/local/bin/wax"
fi

# Stage browser bundle
rm -rf "${ROOTFS_DIR}/usr/local/share/soliloquy/bundle"
mkdir -p "${ROOTFS_DIR}/usr/local/share/soliloquy/bundle"
cp -R "${PROJECT_ROOT}/bundle/." "${ROOTFS_DIR}/usr/local/share/soliloquy/bundle"

echo "Artifacts staged."
