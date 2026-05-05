#!/bin/bash
# build_ui.sh - Build the Soliloquy Servo desktop UI (static bundle for Servo)

set -euo pipefail

echo "=== Soliloquy Servo Desktop UI Build ==="

PROJECT_ROOT=$(pwd)
UI_DIR="$PROJECT_ROOT/ui/desktop"

if [ ! -d "$UI_DIR" ]; then
    echo "Error: Servo desktop UI directory not found at $UI_DIR"
    exit 1
fi

if ! command -v bun &> /dev/null; then
    echo "Error: bun not found."
    exit 1
fi

cd "$UI_DIR"

if [ ! -d "node_modules" ]; then
    echo "[*] Installing dependencies with bun (Svelte v5 bundle)..."
    bun install
fi

echo "[*] Building static bundle for Servo/V8 runtime..."
bun run build

echo "[*] Build complete. Artifacts are in: build/"
echo "    Or point Servo to ui/desktop/build/index.html"
