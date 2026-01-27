#!/bin/bash
# build_common.sh - Shared helper functions for Soliloquy build scripts

set -e

# Common paths and configuration
PROJECT_ROOT=$(pwd)
# The Fuchsia checkout is triple-nested: fuchsia/fuchsia/fuchsia
# The innermost directory contains .jiri_root with fx and jiri binaries
FUCHSIA_DIR="$PROJECT_ROOT/fuchsia/fuchsia/fuchsia"
BOARD_NAME="soliloquy"
BOARD_PATH="boards/arm64/soliloquy"
DEFAULT_PRODUCT="minimal.arm64"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if fx is available and bootstrapped
check_fx_bootstrapped() {
    if [ ! -d "$FUCHSIA_DIR" ]; then
        log_error "Fuchsia directory not found at $FUCHSIA_DIR"
        log_info "Please run ./tools/soliloquy/setup.sh first"
        exit 1
    fi

    if [ ! -f "$FUCHSIA_DIR/scripts/fx-env.sh" ]; then
        log_error "fx-env.sh not found. Fuchsia checkout may be incomplete."
        log_info "Please run ./tools/soliloquy/setup.sh first"
        exit 1
    fi

    # Add jiri and fx to PATH
    export PATH="$FUCHSIA_DIR/.jiri_root/bin:$FUCHSIA_DIR/scripts:$PATH"

    # Source fx-env.sh to get fx functions
    source "$FUCHSIA_DIR/scripts/fx-env.sh"

    # Check if fx command is available
    if ! command -v fx &> /dev/null; then
        log_error "fx command not found. Fuchsia tooling is not bootstrapped."
        log_info "Please run ./tools/soliloquy/setup.sh first"
        exit 1
    fi
}

# Check if current build configuration matches the target
check_build_config() {
    local target_product="$1"
    local target_board="$2"
    
    if [ -f "$FUCHSIA_DIR/out/default/args.gn" ]; then
        local current_product=$(grep "build_info_product" "$FUCHSIA_DIR/out/default/args.gn" 2>/dev/null | cut -d'"' -f2 || echo "")
        local current_board=$(grep "import_board" "$FUCHSIA_DIR/out/default/args.gn" 2>/dev/null | grep -o "//boards/[^']*" | sed 's|//boards/||' || echo "")
        
        if [ "$current_product" = "$target_product" ] && [ "$current_board" = "$target_board" ]; then
            log_info "Build configuration already matches: $target_product with $target_board"
            return 0
        fi
    fi
    
    return 1
}

# Idempotent fx set - only reconfigure if needed
fx_set_idempotent() {
    local product="$1"
    local board="$2"
    local extra_args="$3"
    
    cd "$FUCHSIA_DIR"
    
    if check_build_config "$product" "$board"; then
        log_info "Skipping fx set - configuration already matches"
        return 0
    fi
    
    log_info "Configuring build for $product with $board..."
    
    local fx_set_cmd="fx set $product"
    
    # Add board-specific arguments if not default
    if [ "$board" != "boards/arm64/soliloquy" ]; then
        fx_set_cmd="$fx_set_cmd --board=$board"
    fi
    
    # Add common packages for Soliloquy using --with (universe set)
    # Fuchsia has removed --with-base in favor of assembly overrides,
    # but --with is sufficient for development builds.
    fx_set_cmd="$fx_set_cmd \
        --with //vendor/soliloquy/src/shell:soliloquy_shell \
        --with //vendor/soliloquy/drivers/aic8800:aic8800"
    
    # Add any extra arguments
    if [ -n "$extra_args" ]; then
        fx_set_cmd="$fx_set_cmd $extra_args"
    fi
    
    log_info "Running: $fx_set_cmd"
    eval "$fx_set_cmd"
}

# Emit summary of build artifacts
emit_artifact_summary() {
    local build_type="$1"
    local output_dir="$2"
    
    log_success "Build completed successfully!"
    echo ""
    log_info "=== Build Artifact Summary ==="
    log_info "Build Type: $build_type"
    log_info "Output Directory: $output_dir"
    
    if [ -d "$output_dir" ]; then
        echo ""
        log_info "Key Artifacts:"
        find "$output_dir" -name "*.far" -o -name "*.zbi" -o -name "*.img" -o -name "*.elf" | head -10 | while read -r artifact; do
            local size=$(stat -f%z "$artifact" 2>/dev/null || stat -c%s "$artifact" 2>/dev/null || echo "unknown")
            log_info "  $(basename "$artifact") (${size} bytes)"
        done
        
        local total_artifacts=$(find "$output_dir" -type f | wc -l)
        log_info "Total files: $total_artifacts"
    else
        log_warning "Output directory not found: $output_dir"
    fi
    echo ""
}

# Get output directory for different build types
get_output_dir() {
    local build_type="$1"
    
    case "$build_type" in
        "fuchsia")
            echo "$FUCHSIA_DIR/out/default"
            ;;
        "sdk")
            echo "$PROJECT_ROOT/out/arm64"
            ;;
        "bazel")
            echo "$PROJECT_ROOT/bazel-bin"
            ;;
        *)
            log_error "Unknown build type: $build_type"
            exit 1
            ;;
    esac
}