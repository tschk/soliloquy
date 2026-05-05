#!/bin/bash
# dev_ui.sh - Development helper for Soliloquy Servo desktop UI (Svelte v5)

set -euo pipefail

echo "=== Soliloquy Servo Desktop UI Development ==="

PROJECT_ROOT=$(pwd)
UI_DIR="$PROJECT_ROOT/ui/desktop"

if [ ! -d "$UI_DIR" ]; then
    echo "Error: Servo desktop UI directory not found at $UI_DIR"
    echo "Please ensure you're running this from the project root"
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

echo "[*] Starting Svelte dev server for the Servo/V8 desktop surface..."
echo "Press Ctrl+C to stop the development server."

PORT=${VITE_PORT:-5173}
bun run dev -- --host 0.0.0.0 --port "$PORT"
