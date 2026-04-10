#!/bin/sh
set -eu

OUT_DIR="${1:-build/alpine/qemu}"
ALPINE_VERSION="${ALPINE_VERSION:-3.21}"
ARCH="${QEMU_ARCH:-x86_64}"

mkdir -p "${OUT_DIR}"

BASE="https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VERSION}/releases/${ARCH}/netboot"

curl -fsSL "${BASE}/vmlinuz-virt" -o "${OUT_DIR}/vmlinuz-virt"

echo "Fetched QEMU kernel artifacts into ${OUT_DIR}"
