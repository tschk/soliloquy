#!/bin/bash
# env.sh - Quick environment setup for Soliloquy Alpine/Linux development
# Usage: source env.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    echo "Error: This script must be sourced, not executed."
    echo "Usage: source env.sh"
    exit 1
fi

echo "=== Setting up Soliloquy Development Environment ==="

export SOLILOQUY_ROOT="$SCRIPT_DIR"
export PATH="$SOLILOQUY_ROOT/tools:$SOLILOQUY_ROOT/system/alpine/scripts:$PATH"

SERVO_DIR="$SOLILOQUY_ROOT/third_party/servo"
if [ -d "$SERVO_DIR" ]; then
    export SERVO_DIR
    echo "[✓] Servo browser engine found at $SERVO_DIR"
fi

if [ -d "$SOLILOQUY_ROOT/build/alpine/rootfs" ]; then
    echo "[✓] Alpine rootfs present at build/alpine/rootfs"
fi

echo ""
echo "Environment ready! Useful commands:"
echo "  build-rootfs.sh         - Build the Alpine root filesystem"
echo "  build-qemu-image.sh     - Create the Alpine/QEMU image"
echo "  run-qemu.sh             - Launch the Alpine/QEMU environment"
echo ""
