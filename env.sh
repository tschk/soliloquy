#!/bin/bash
# env.sh - Quick environment setup for Soliloquy development
# Usage: source env.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check if we're being sourced
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    echo "Error: This script must be sourced, not executed."
    echo "Usage: source env.sh"
    exit 1
fi

echo "=== Setting up Soliloquy Development Environment ==="

# Set up project root
export SOLILOQUY_ROOT="$SCRIPT_DIR"
export FUCHSIA_DIR="$SOLILOQUY_ROOT/fuchsia/fuchsia/fuchsia"

# Check if Fuchsia is set up
if [ ! -d "$FUCHSIA_DIR" ]; then
    echo "Warning: Fuchsia directory not found."
    echo "Run ./tools/soliloquy/setup.sh to initialize the build environment."
    return 1
fi

# Add Fuchsia tools to PATH
if [ -d "$FUCHSIA_DIR/.jiri_root/bin" ]; then
    export PATH="$FUCHSIA_DIR/.jiri_root/bin:$FUCHSIA_DIR/scripts:$PATH"
    echo "[✓] Added jiri and fx tools to PATH"
fi

# Source Fuchsia environment
if [ -f "$FUCHSIA_DIR/scripts/fx-env.sh" ]; then
    source "$FUCHSIA_DIR/scripts/fx-env.sh"
    echo "[✓] Fuchsia environment loaded"
else
    echo "[!] Warning: fx-env.sh not found"
fi

# Add Soliloquy tools to PATH
export PATH="$SOLILOQUY_ROOT/tools/soliloquy:$PATH"
echo "[✓] Added Soliloquy tools to PATH"

# Servo path
export SERVO_DIR="$SOLILOQUY_ROOT/vendor/servo"
if [ -d "$SERVO_DIR" ]; then
    echo "[✓] Servo browser engine found at $SERVO_DIR"
fi

echo ""
echo "Environment ready! Available commands:"
echo "  fx          - Fuchsia build tool"
echo "  build.sh    - Build Soliloquy"
echo "  setup.sh    - Run full setup"
echo ""
