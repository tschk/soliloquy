#!/usr/bin/env bash
# Soliloquy Build Orchestrator
# Main entry point for all build operations
#
# Usage: ./scripts/build.sh [target] [options]
#
# Targets:
#   all       - Build everything (default)
#   backend   - Build V backend only
#   ui        - Build Svelte UI only
#   shell     - Build Servo shell
#
# Options:
#   --release  - Release build (default: debug)
#   --clean    - Clean before building
#   --test     - Run tests after build
#   --help     - Show this help

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Source shared library
source "${SCRIPT_DIR}/lib/common.sh"

# Defaults
TARGET="${1:-all}"
RELEASE_MODE=false
CLEAN_FIRST=false
RUN_TESTS=false

# Parse options
shift || true
while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            RELEASE_MODE=true
            shift
            ;;
        --clean)
            CLEAN_FIRST=true
            shift
            ;;
        --test)
            RUN_TESTS=true
            shift
            ;;
        --help|-h)
            head -24 "$0" | tail -n +2 | sed 's/^# //' | sed 's/^#//'
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Build functions
build_backend() {
    log_info "Building V backend..."
    cd "${PROJECT_ROOT}/backend"
    
    if ! command -v v &> /dev/null; then
        log_error "V compiler not found. Install from https://vlang.io"
        exit 1
    fi
    
    if $CLEAN_FIRST; then
        rm -rf "${PROJECT_ROOT}/backend/soliloquy" 2>/dev/null || true
    fi
    
    local v_flags=""
    if $RELEASE_MODE; then
        v_flags="-prod"
    fi
    
    v $v_flags build .
    log_success "Backend built"
    
    if $RUN_TESTS; then
        log_info "Running backend tests..."
        v test .
        log_success "Backend tests passed"
    fi
}

build_ui() {
    log_info "Building Svelte UI..."
    cd "${PROJECT_ROOT}/ui/desktop"
    
    if ! command -v pnpm &> /dev/null; then
        log_error "pnpm not found. Install with: corepack enable pnpm"
        exit 1
    fi
    
    if $CLEAN_FIRST; then
        rm -rf node_modules .svelte-kit build 2>/dev/null || true
    fi
    
    pnpm install
    
    if $RELEASE_MODE; then
        pnpm build
    else
        log_info "UI ready for dev mode (run: pnpm dev)"
    fi
    
    log_success "UI built"
}

build_shell() {
    log_info "Building Servo shell with Cargo..."
    cd "${PROJECT_ROOT}"

    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Install Rust first."
        exit 1
    fi
    
    if $CLEAN_FIRST; then
        cargo clean --manifest-path src/shell/Cargo.toml
    fi
    
    local cargo_flags="--manifest-path src/shell/Cargo.toml"
    if $RELEASE_MODE; then
        cargo_flags="$cargo_flags --release"
    fi

    cargo build $cargo_flags
    log_success "Shell built"
    
    if $RUN_TESTS; then
        log_info "Running shell tests..."
        cargo test $cargo_flags --lib
        log_success "Shell tests passed"
    fi
}

build_all() {
    log_info "Building all targets..."
    build_backend
    build_ui
    build_shell
    log_success "All targets built successfully"
}

# Main
log_info "=== Soliloquy Build ==="
log_info "Target: $TARGET"
log_info "Release: $RELEASE_MODE"

case "$TARGET" in
    all)
        build_all
        ;;
    backend)
        build_backend
        ;;
    ui)
        build_ui
        ;;
    shell)
        build_shell
        ;;
    *)
        log_error "Unknown target: $TARGET"
        echo "Valid targets: all, backend, ui, shell"
        exit 1
        ;;
esac

log_success "Build complete!"
