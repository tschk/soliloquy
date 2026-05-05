#!/usr/bin/env bash
# Start Soliloquy in appropriate mode (desktop or headless)
# This script just starts the backend and optionally the UI for dev purposes

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BACKEND_DIR="${PROJECT_ROOT}/backend"
UI_DIR="${PROJECT_ROOT}/ui/desktop"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[soliloquy]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[soliloquy]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[soliloquy]${NC} $1"
}

log_error() {
    echo -e "${RED}[soliloquy]${NC} $1"
}

# Check if we're in dev mode (not on a packaged device image)
check_dev_mode() {
    if [[ "$OSTYPE" == "darwin"* ]] || [[ "$OSTYPE" == "linux-gnu"* ]]; then
        return 0
    fi
    
    return 1
}

# Start backend
start_backend() {
    log_info "Starting Soliloquy backend (V)..."
    
    if ! command -v v &> /dev/null; then
        log_error "V language not found. Install from https://vlang.io"
        exit 1
    fi
    
    cd "${BACKEND_DIR}"
    
    # Check if .env exists
    if [[ ! -f .env ]]; then
        log_warn "No .env file found. Copy .env.example and configure."
        log_warn "Backend will start with defaults, but OAuth won't work."
    fi
    
    # Load .env if it exists
    if [[ -f .env ]]; then
        set -a
        source .env
        set +a
    fi
    
    v run . &
    BACKEND_PID=$!
    
    log_success "Backend started (PID: ${BACKEND_PID})"
    echo "${BACKEND_PID}" > /tmp/soliloquy-backend.pid
    
    # Wait for backend to be ready
    log_info "Waiting for backend to be ready..."
    for i in {1..30}; do
        if curl -s http://localhost:3030/health > /dev/null 2>&1; then
            log_success "Backend ready"
            return 0
        fi
        sleep 1
    done
    
    log_error "Backend failed to start"
    return 1
}

# Start UI in desktop mode
start_ui() {
    log_info "Starting Soliloquy UI (Svelte + Servo)..."
    
    cd "${UI_DIR}"
    
    if ! command -v bun &> /dev/null; then
        log_error "bun not found."
        exit 1
    fi
    
    # Install dependencies if needed
    if [[ ! -d node_modules ]]; then
        log_info "Installing UI dependencies..."
        bun install
    fi
    
    # Start dev server
    bun run dev &
    UI_PID=$!
    
    log_success "UI dev server started (PID: ${UI_PID})"
    echo "${UI_PID}" > /tmp/soliloquy-ui.pid
    
    log_success "🚀 Soliloquy running at http://localhost:5173"
}

# Cleanup on exit
cleanup() {
    log_info "Stopping Soliloquy..."
    
    if [[ -f /tmp/soliloquy-ui.pid ]]; then
        UI_PID=$(cat /tmp/soliloquy-ui.pid)
        kill "${UI_PID}" 2>/dev/null || true
        rm /tmp/soliloquy-ui.pid
    fi
    
    if [[ -f /tmp/soliloquy-backend.pid ]]; then
        BACKEND_PID=$(cat /tmp/soliloquy-backend.pid)
        kill "${BACKEND_PID}" 2>/dev/null || true
        rm /tmp/soliloquy-backend.pid
    fi
    
    log_success "Soliloquy stopped"
}

trap cleanup EXIT INT TERM

# Main
main() {
    log_info "🌟 Starting Soliloquy..."
    
    # Always start backend
    start_backend
    
    # In dev mode, start UI as well.
    if check_dev_mode; then
        log_info "🔧 Dev mode detected - starting UI dev server"
        start_ui
    fi
    
    # Keep running
    log_success "✨ Soliloquy is ready"
    log_info "Press Ctrl+C to stop"
    
    wait
}

main "$@"
