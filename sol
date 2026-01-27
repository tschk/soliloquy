#!/bin/bash
# Soliloquy CLI Wrapper
# Usage: ./sol [command] [args...]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOOLS_DIR="$SCRIPT_DIR/tools/soliloquy"

# Load environment
if [ -f "$SCRIPT_DIR/env.sh" ]; then
    source "$SCRIPT_DIR/env.sh" > /dev/null 2>&1
fi

function show_help() {
    echo "Soliloquy Development CLI"
    echo "Usage: sol <command> [options]"
    echo ""
    echo "Commands:"
    echo "  build       Build the Soliloquy images (wraps build.sh)"
    echo "  setup       Set up the development environment (wraps setup.sh)"
    echo "  clean       Clean build artifacts (fx clean)"
    echo "  run         Run in QEMU (wraps run_qemu.sh)"
    echo "  flash       Flash to device (wraps flash.sh)"
    echo "  validate    Validate manifests (wraps validate_manifest.sh)"
    echo "  help        Show this help message"
    echo ""
    echo "Examples:"
    echo "  sol build"
    echo "  sol run"
}

if [ $# -eq 0 ]; then
    show_help
    exit 1
fi

COMMAND="$1"
shift

case "$COMMAND" in
    build)
        "$TOOLS_DIR/build.sh" "$@"
        ;;
    setup)
        "$TOOLS_DIR/setup.sh" "$@"
        ;;
    clean)
        # Assuming we are in the root or can configure fx
        if command -v fx &> /dev/null; then
            fx clean
        else
            echo "Error: 'fx' command not found. Have you run 'sol setup'?"
            exit 1
        fi
        ;;
    run)
        "$TOOLS_DIR/run_qemu.sh" "$@"
        ;;
    flash)
        "$TOOLS_DIR/flash.sh" "$@"
        ;;
    validate)
        "$TOOLS_DIR/validate_manifest.sh" "$@"
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo "Unknown command: $COMMAND"
        show_help
        exit 1
        ;;
esac
