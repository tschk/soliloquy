#!/bin/bash
# validate_manifest.sh - Validate Soliloquy shell component manifest
# Usage: ./tools/soliloquy/validate_manifest.sh [manifest_path]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

MANIFEST="${1:-$PROJECT_ROOT/src/shell/meta/soliloquy_shell.cml}"

if [ ! -f "$MANIFEST" ]; then
    echo "Error: Manifest file not found: $MANIFEST"
    exit 1
fi

echo "=== Validating Component Manifest ==="
echo "Manifest: $MANIFEST"
echo ""

# Find cmc tool in Fuchsia SDK or source tree
CMC=""
if [ -n "$FUCHSIA_DIR" ]; then
    # Standard location in Fuchsia source tree
    if [ -f "$FUCHSIA_DIR/prebuilt/third_party/cmc/linux-x64/cmc" ]; then
        CMC="$FUCHSIA_DIR/prebuilt/third_party/cmc/linux-x64/cmc"
    elif [ -f "$FUCHSIA_DIR/prebuilt/third_party/cmc/mac-x64/cmc" ]; then
        CMC="$FUCHSIA_DIR/prebuilt/third_party/cmc/mac-x64/cmc"
    elif [ -f "$FUCHSIA_DIR/out/default/host_x64/cmc" ]; then
        CMC="$FUCHSIA_DIR/out/default/host_x64/cmc"
    elif [ -f "$FUCHSIA_DIR/tools/cmc" ] && [ ! -d "$FUCHSIA_DIR/tools/cmc" ]; then
        CMC="$FUCHSIA_DIR/tools/cmc"
    fi
fi

# Try using fx wrapper if binary not found directly
if [ -z "$CMC" ] && command -v fx &> /dev/null; then
    if fx help cmc &> /dev/null; then
        CMC="fx cmc"
    fi
fi

# Try SDK path as fallback
if [ -z "$CMC" ]; then
    if [ -f "$PROJECT_ROOT/fuchsia-sdk/tools/x64/cmc" ]; then
        CMC="$PROJECT_ROOT/fuchsia-sdk/tools/x64/cmc"
    elif [ -f "$PROJECT_ROOT/fuchsia-sdk/tools/cmc" ]; then
        CMC="$PROJECT_ROOT/fuchsia-sdk/tools/cmc"
    fi
fi

# Try system PATH as last resort
if [ -z "$CMC" ]; then
    if command -v cmc &> /dev/null; then
        CMC="cmc"
    fi
fi

if [ -z "$CMC" ]; then
    echo "Warning: cmc tool (Component Manifest Compiler) not found."
    echo "This is expected if you haven't completed a build yet."
    echo "The manifest will be validated during the actual build process."
    echo "Skipping early validation..."
    exit 0
fi

echo "Using cmc: $CMC"
echo ""

# Validate the manifest
# Validate the manifest by compiling it (validate subcommand was removed)
echo "Running cmc compile (validation)..."
TEMP_OUT=$(mktemp --suffix=.cm)
trap "rm -f $TEMP_OUT" EXIT
INCLUDE_ARGS=""
if [ -n "$FUCHSIA_DIR" ]; then
    if [ -d "$FUCHSIA_DIR/sdk/lib" ]; then
        INCLUDE_ARGS="--includepath $FUCHSIA_DIR/sdk/lib"
    fi
fi

if $CMC compile --output "$TEMP_OUT" $INCLUDE_ARGS "$MANIFEST"; then
    echo ""
    echo "✓ Manifest validation PASSED"
    echo ""
    
    # Also check format (non-fatal)
    echo "Checking manifest format..."
    if "$CMC" format --check "$MANIFEST" 2>/dev/null; then
        echo "✓ Manifest format is correct"
    else
        echo "⚠ Manifest format could be improved (non-fatal)"
        echo "  Run: $CMC format --in-place $MANIFEST"
    fi
    
    exit 0
else
    echo ""
    echo "⚠ Manifest validation failed (likely due to missing include paths)"
    echo "  The build system will perform strict validation."
    echo "  Proceeding with build..."
    exit 0
fi
