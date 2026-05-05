#!/usr/bin/env bash
# Soliloquy Development Mode
# Starts shell and UI with hot reload
#
# Usage: ./scripts/dev.sh [options]
#
# Options:
#   --shell-only    Start only the shell
#   --ui-only        Start only the UI dev server
#   --qemu           Build and run in QEMU (Linux target)
#   --help           Show this help

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Source shared library
source "${SCRIPT_DIR}/lib/common.sh"

# Defaults
SHELL_ONLY=false
UI_ONLY=false
QEMU_BUILD=false
SHELL_PID=""
UI_PID=""

# Parse options
while [[ $# -gt 0 ]]; do
    case $1 in
        --shell-only)
            SHELL_ONLY=true
            shift
            ;;
        --ui-only)
            UI_ONLY=true
            shift
            ;;
        --qemu)
            QEMU_BUILD=true
            shift
            ;;
        --help|-h)
            head -14 "$0" | tail -n +2 | sed 's/^# //' | sed 's/^#//'
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Cleanup on exit
cleanup() {
    log_info "Stopping Soliloquy dev mode..."
    
    if [[ -n "$UI_PID" ]]; then
        kill "$UI_PID" 2>/dev/null || true
    fi
    
    if [[ -n "$SHELL_PID" ]]; then
        kill "$SHELL_PID" 2>/dev/null || true
    fi
    
    # Clean up PID files
    rm -f /tmp/soliloquy-shell.pid /tmp/soliloquy-ui.pid
    
    log_success "Stopped"
}

trap cleanup EXIT INT TERM

start_shell() {
    log_info "Starting Rust shell..."
    
    cd "${PROJECT_ROOT}"
    
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Install Rust from https://rustup.rs"
        exit 1
    fi
    
    # Load .env if exists
    if [[ -f .env ]]; then
        log_info "Loading .env configuration"
        set -a
        source .env
        set +a
    fi
    
    # Start shell
    cargo run --bin soliloquy_shell &
    SHELL_PID=$!
    echo "$SHELL_PID" > /tmp/soliloquy-shell.pid
    
    log_success "Shell started"
}

# Start UI dev server
start_ui() {
    log_info "Starting Svelte UI dev server..."
    
    cd "${PROJECT_ROOT}/ui/desktop"
    
    if ! command -v bun &> /dev/null; then
        log_error "bun not found."
        exit 1
    fi
    
    # Install deps if needed
    if [[ ! -d node_modules ]]; then
        log_info "Installing UI dependencies..."
        bun install
    fi
    
    bun run dev &
    UI_PID=$!
    echo "$UI_PID" > /tmp/soliloquy-ui.pid
    
    log_success "UI dev server started"
    log_success "🚀 Open http://localhost:5173"
}

start_qemu() {
    log_info "Building for QEMU (Linux x86_64 gnu target)..."
    
    cd "${PROJECT_ROOT}"
    
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Install Rust from https://rustup.rs"
        exit 1
    fi
    
    # Add linux target if not present
    if ! rustup target list --installed | grep -q "x86_64-unknown-linux-gnu"; then
        log_info "Adding Linux gnu target..."
        rustup target add x86_64-unknown-linux-gnu
    fi
    
    # Build for Linux gnu
    log_info "Building release binary for Linux gnu..."
    export CC_x86_64_unknown_linux_gnu="$PROJECT_ROOT/scripts/zig-cc-wrapper.sh"
    export RING_TARGET_TRIPLE=x86_64-linux-gnu
    export AWS_LC_SYS_TARGET_TRIPLE=x86_64-linux-gnu
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$PROJECT_ROOT/scripts/zig-cc-wrapper.sh"
    cargo build --release --target x86_64-unknown-linux-gnu --bin soliloquy_shell
    
    log_success "Built soliloquy_shell for QEMU (gnu)"
    log_info "To run in QEMU, set up a Linux VM image and copy target/x86_64-unknown-linux-gnu/release/soliloquy_shell"
    log_info "Example: Use Ubuntu ISO and install the binary"
}

# Main
main() {
    log_info "🌟 Soliloquy Development Mode"
    
    if $QEMU_BUILD; then
        start_qemu
    elif $UI_ONLY; then
        start_ui
    elif $SHELL_ONLY; then
        start_shell
    else
        start_shell
        start_ui
    fi
    
    echo ""
    log_success "✨ Development servers running"
    if ! $SHELL_ONLY && ! $QEMU_BUILD; then
        log_info "Shell: Running"
    fi
    if ! $UI_ONLY && ! $QEMU_BUILD; then
        log_info "Frontend: http://localhost:5173"
    fi
    log_info ""
    log_info "Press Ctrl+C to stop"
    
    wait
}

main
