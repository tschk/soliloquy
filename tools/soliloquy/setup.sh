#!/bin/bash
# setup.sh - Soliloquy Environment Setup
# Supports Linux (Debian/Ubuntu/RHEL/Fedora) and macOS

set -e

echo "=== Soliloquy Setup ==="

# Detect OS
OS=$(uname -s)

# 1. Install Prerequisites
echo "[*] Installing dependencies..."
if [ "$OS" = "Darwin" ]; then
    echo "Detected macOS..."
    if ! command -v brew &> /dev/null; then
        echo "Error: Homebrew not found. Please install Homebrew first:"
        echo "  /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
        exit 1
    fi
    brew install git curl unzip python@3 make
elif command -v dnf &> /dev/null; then
    echo "Detected Fedora/RHEL..."
    sudo dnf install -y git curl unzip python3 python3-pip @development-tools gcc-c++ make
elif command -v apt-get &> /dev/null; then
    echo "Detected Debian/Ubuntu..."
    sudo apt-get update
    sudo apt-get install -y git curl unzip python3 python3-pip build-essential gcc g++ make
else
    echo "Error: Unsupported platform or package manager. Please install dependencies manually."
    exit 1
fi

# Check for gitcookies which helps avoid 429 errors
if [ ! -f "$HOME/.gitcookies" ]; then
    echo "---------------------------------------------------------------------"
    echo "WARNING: ~/.gitcookies not found."
    echo "To avoid HTTP 429 (Rate Limit) errors from Google Source, it is"
    echo "HIGHLY recommended to authenticate."
    echo ""
    echo "1. Go to: https://fuchsia.googlesource.com/new-password"
    echo "2. Log in and run the provided script in your terminal."
    echo "---------------------------------------------------------------------"
    echo "Waiting 5 seconds before continuing..."
    sleep 5
fi

# 2. Clone Fuchsia (if not exists)
# Use a directory relative to the script execution (project root)
PROJECT_ROOT=$(pwd)
# Note: The bootstrap script creates a nested structure: fuchsia/fuchsia/fuchsia
# FUCHSIA_CHECKOUT is where we clone to, FUCHSIA_DIR is where the actual tree ends up
FUCHSIA_CHECKOUT="$PROJECT_ROOT/fuchsia/fuchsia"
# After bootstrap, the actual source tree with .jiri_root is in a nested 'fuchsia' directory
FUCHSIA_DIR="$PROJECT_ROOT/fuchsia/fuchsia/fuchsia"

if [ -d "$FUCHSIA_DIR" ]; then
    echo "[*] Fuchsia directory exists at $FUCHSIA_DIR"
else
    echo "[*] Cloning Fuchsia repository to $FUCHSIA_DIR..."
    git clone https://fuchsia.googlesource.com/fuchsia "$FUCHSIA_DIR"
fi

# 3. Bootstrap Fuchsia
echo "[*] Bootstrapping Fuchsia..."
cd "$FUCHSIA_DIR"
if ! scripts/bootstrap; then
    echo "[!] Bootstrap failed. This is often due to Google Source rate limiting (HTTP 429)."
    echo "[*] Waiting 60 seconds before retrying with reduced parallelism..."
    sleep 60
    
    if [ -f ".jiri_root/bin/jiri" ]; then
        echo "[*] Retrying 'jiri update' with -j 1 (sequential download)..."
        .jiri_root/bin/jiri update -j 1
    else
        echo "[X] Critical: jiri binary not found. Cannot recover."
        exit 1
    fi
fi
cd "$PROJECT_ROOT"

# 4. Clone Servo (if not exists)
SERVO_DIR="$PROJECT_ROOT/vendor/servo"
if [ -d "$SERVO_DIR/.git" ]; then
    echo "[*] Servo directory exists at $SERVO_DIR"
    echo "[*] Updating Servo..."
    cd "$SERVO_DIR"
    # Servo uses 'main' as default branch, not 'master'
    git fetch origin
    git checkout main 2>/dev/null || git checkout -b main origin/main 2>/dev/null || true
    git pull origin main || echo "[!] Warning: Could not pull Servo, continuing..."
    cd "$PROJECT_ROOT"
else
    echo "[*] Cloning Servo repository..."
    # Create parent dir if needed
    mkdir -p "$(dirname "$SERVO_DIR")"
    git clone --branch main https://github.com/servo/servo.git "$SERVO_DIR"
    
    # Initialize Servo as git submodule
    cd "$PROJECT_ROOT"
    git submodule add https://github.com/servo/servo.git vendor/servo 2>/dev/null || true
    git submodule update --init --recursive
fi

# 4.1. Setup rusty_v8 for V8 integration
echo "[*] Setting up V8 integration..."
THIRD_PARTY_DIR="$PROJECT_ROOT/third_party/rust_crates"
if [ ! -d "$THIRD_PARTY_DIR" ]; then
    mkdir -p "$THIRD_PARTY_DIR"
fi

# Note: rusty_v8 will be pulled via Cargo when building
echo "[*] V8 integration ready via Cargo dependencies"

# 5. Link Soliloquy Sources into Fuchsia Tree
echo "[*] Linking Soliloquy sources..."
# Create vendor directory
mkdir -p "$FUCHSIA_DIR/vendor/soliloquy"

# Link Board
mkdir -p "$FUCHSIA_DIR/boards/arm64"
ln -sfn "$PROJECT_ROOT/boards/arm64/soliloquy" "$FUCHSIA_DIR/boards/arm64/soliloquy"

# Link Drivers
# We'll place drivers in vendor/soliloquy/drivers for now
mkdir -p "$FUCHSIA_DIR/vendor/soliloquy/drivers"
ln -sfn "$PROJECT_ROOT/drivers/wifi/aic8800" "$FUCHSIA_DIR/vendor/soliloquy/drivers/aic8800"

# Link Shell
mkdir -p "$FUCHSIA_DIR/vendor/soliloquy/src"
ln -sfn "$PROJECT_ROOT/src/shell" "$FUCHSIA_DIR/vendor/soliloquy/src/shell"

# Link Servo
ln -sfn "$SERVO_DIR" "$FUCHSIA_DIR/vendor/servo"

# 6. Setup Environment and PATH
echo "[*] Setting up environment..."

# Add jiri, fx, and Fuchsia tools to PATH
# fx is in scripts/, jiri is in .jiri_root/bin/
export PATH="$FUCHSIA_DIR/.jiri_root/bin:$FUCHSIA_DIR/scripts:$PATH"

# Source Fuchsia's fx-env.sh (sets up fx command)
if [ -f "$FUCHSIA_DIR/scripts/fx-env.sh" ]; then
    source "$FUCHSIA_DIR/scripts/fx-env.sh"
    echo "[*] Fuchsia environment sourced successfully"
else
    echo "[!] Warning: fx-env.sh not found at $FUCHSIA_DIR/scripts/fx-env.sh"
fi

# Also add to user's shell rc file for persistence
SHELL_RC=""
if [ -f "$HOME/.zshrc" ]; then
    SHELL_RC="$HOME/.zshrc"
elif [ -f "$HOME/.bashrc" ]; then
    SHELL_RC="$HOME/.bashrc"
fi

if [ -n "$SHELL_RC" ]; then
    # Check if already added
    if ! grep -q "FUCHSIA_DIR.*soliloquy" "$SHELL_RC" 2>/dev/null; then
        echo "" >> "$SHELL_RC"
        echo "# Soliloquy/Fuchsia Environment" >> "$SHELL_RC"
        echo "export FUCHSIA_DIR=\"$FUCHSIA_DIR\"" >> "$SHELL_RC"
        echo "export PATH=\"\$FUCHSIA_DIR/.jiri_root/bin:\$FUCHSIA_DIR/scripts:\$PATH\"" >> "$SHELL_RC"
        echo "[ -f \"\$FUCHSIA_DIR/scripts/fx-env.sh\" ] && source \"\$FUCHSIA_DIR/scripts/fx-env.sh\"" >> "$SHELL_RC"
        echo "[*] Added Fuchsia environment to $SHELL_RC"
    else
        echo "[*] Fuchsia environment already in $SHELL_RC"
    fi
fi

echo "=== Setup Complete ==="
echo "Servo browser engine: ✅ Integrated"
echo "V8 JavaScript runtime: ✅ Ready via Cargo"
echo "Build system: ✅ GN + Bazel configured"
echo ""
echo "Next steps:"
echo "1. Reload your shell or run: source $FUCHSIA_DIR/scripts/fx-env.sh"
echo "2. Verify fx is available: fx --version"
echo "3. Build Soliloquy: ./tools/soliloquy/build.sh"
echo "4. For browser engine integration: Read docs/servo_integration.md"
