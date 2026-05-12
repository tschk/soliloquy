#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
UI_DIR="${PROJECT_ROOT}/ui/desktop"
CREPUSCULARITY_GPUI_DIR="${PROJECT_ROOT}/../crepuscularity/crates/crepuscularity-gpui"

require_command() {
    if ! command -v "$1" > /dev/null 2>&1; then
        echo "Error: $1 not found."
        exit 1
    fi
}

require_command bun
require_command cargo

if [[ ! -d "${UI_DIR}" ]]; then
    echo "Error: UI directory not found at ${UI_DIR}"
    exit 1
fi

if [[ ! -d "${CREPUSCULARITY_GPUI_DIR}" ]]; then
    echo "Error: crepuscularity GPUI crate not found at ${CREPUSCULARITY_GPUI_DIR}"
    exit 1
fi

cd "${UI_DIR}"

if [[ ! -d node_modules ]]; then
    bun install
fi

bun run check
bun run build

cd "${PROJECT_ROOT}"

cargo check -p soliloquy-shell --bin soliloquy_desktop --features gpui
