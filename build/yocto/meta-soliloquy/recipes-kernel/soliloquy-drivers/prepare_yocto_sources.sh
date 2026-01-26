#!/bin/bash
# prepare_yocto_sources.sh - Prepare driver sources for Yocto builds
# This script copies the driver sources into the BitBake recipe files directory

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Navigate from recipes-kernel/soliloquy-drivers to the project root
# Path: build/yocto/meta-soliloquy/recipes-kernel/soliloquy-drivers -> soliloquy root
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../../.." && pwd)"
RECIPE_DIR="$SCRIPT_DIR"

echo "=== Preparing Yocto Driver Sources ==="
echo "Project root: $PROJECT_ROOT"
echo "Recipe dir: $RECIPE_DIR"

# AIC8800 Driver
AIC8800_FILES="$RECIPE_DIR/../aic8800-driver/files"
echo "[*] Preparing AIC8800 driver sources..."
mkdir -p "$AIC8800_FILES"
rm -rf "$AIC8800_FILES/aic8800_fdrv" "$AIC8800_FILES/aic_load_fw" 2>/dev/null || true

# Copy driver sources
cp "$PROJECT_ROOT/drivers/wifi/aic8800/linux_reference/Makefile" "$AIC8800_FILES/"
cp "$PROJECT_ROOT/drivers/wifi/aic8800/linux_reference/Kconfig" "$AIC8800_FILES/"
cp -r "$PROJECT_ROOT/drivers/wifi/aic8800/linux_reference/aic8800_fdrv" "$AIC8800_FILES/"
cp -r "$PROJECT_ROOT/drivers/wifi/aic8800/linux_reference/aic_load_fw" "$AIC8800_FILES/"

# Copy firmware files
mkdir -p "$AIC8800_FILES/firmware"
cp -r "$PROJECT_ROOT/drivers/wifi/aic8800/firmware/"* "$AIC8800_FILES/firmware/" 2>/dev/null || echo "[!] No firmware files found"

echo "[✓] AIC8800 driver sources prepared"

# Soliloquy Drivers (full collection)
SOLILOQUY_FILES="$RECIPE_DIR/files"
echo "[*] Preparing full Soliloquy driver sources..."
mkdir -p "$SOLILOQUY_FILES/drivers"
rm -rf "$SOLILOQUY_FILES/drivers/"* 2>/dev/null || true

# Copy all driver directories
cp -r "$PROJECT_ROOT/drivers/"* "$SOLILOQUY_FILES/drivers/"

echo "[✓] Soliloquy driver sources prepared"

echo ""
echo "=== Done ==="
echo "You can now build with: bitbake aic8800-driver"
echo "Or for all drivers: bitbake soliloquy-drivers"
