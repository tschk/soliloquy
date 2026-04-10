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

PNPM_HOME="${PNPM_HOME:-$HOME/.local/share/pnpm}"

if command -v pnpm &> /dev/null; then
    PNPM="pnpm"
elif [ -x "${PNPM_HOME}/pnpm" ]; then
    export PATH="${PNPM_HOME}:${PATH}"
    PNPM="pnpm"
elif command -v corepack &> /dev/null; then
    PNPM="corepack pnpm"
elif command -v curl &> /dev/null; then
    echo "[*] Installing pnpm with the official installer..."
    export PNPM_HOME
    curl -fsSL https://get.pnpm.io/install.sh | sh -
    export PATH="${PNPM_HOME}:${PATH}"
    PNPM="pnpm"
else
    echo "Error: pnpm not found and no installer path is available."
    exit 1
fi

cd "$UI_DIR"

if [ ! -d "node_modules" ]; then
    echo "[*] Installing dependencies with pnpm (Svelte v5 bundle)..."
    ${PNPM} install
fi

echo "[*] Building static bundle for Servo/V8 runtime..."
${PNPM} build

echo "[*] Build complete. Artifacts are in: build/"
echo "    Serve with: pnpm dlx serve build -l 4173"
echo "    Or point Servo to ui/desktop/build/index.html"
