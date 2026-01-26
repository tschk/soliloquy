#!/bin/bash
# fx-env.sh - Soliloquy wrapper for Fuchsia fx-env
# Source this file to set up the Fuchsia build environment

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR/.."

# The actual Fuchsia checkout is triple-nested
FUCHSIA_DIR="$PROJECT_ROOT/fuchsia/fuchsia/fuchsia"

if [ ! -d "$FUCHSIA_DIR" ]; then
    echo "Error: Fuchsia directory not found at $FUCHSIA_DIR"
    echo "Please run ./tools/soliloquy/setup.sh first"
    return 1 2>/dev/null || exit 1
fi

# Add jiri tools and fx to PATH
export PATH="$FUCHSIA_DIR/.jiri_root/bin:$FUCHSIA_DIR/scripts:$PATH"

# Source Fuchsia's fx-env.sh
if [ -f "$FUCHSIA_DIR/scripts/fx-env.sh" ]; then
    source "$FUCHSIA_DIR/scripts/fx-env.sh"
    echo "Fuchsia environment loaded. fx is now available."
    echo "Run 'fx --version' to verify."
else
    echo "Warning: fx-env.sh not found at $FUCHSIA_DIR/scripts/fx-env.sh"
    echo "Fuchsia checkout may be incomplete."
    return 1 2>/dev/null || exit 1
fi

# Export useful Soliloquy variables
export SOLILOQUY_ROOT="$PROJECT_ROOT"
export FUCHSIA_DIR="$FUCHSIA_DIR"
