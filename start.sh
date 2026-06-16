#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
ALPENGLOW_OS="$(CDPATH='' cd -- "${ROOT_DIR}/../alpenglow-os" && pwd)"
QEMU_RUN="${QEMU_RUN:-1}"
QEMU_HEADLESS="${QEMU_HEADLESS:-0}"
QEMU_ARCH="${QEMU_ARCH:-aarch64}"

usage() {
  cat <<'USAGE'
Usage: ./start.sh [options]

Build the Soliloquy desktop Alpine image and boot it in QEMU.

Options:
  --build-only  Prepare QEMU artifacts, skip VM launch
  --headless    Run QEMU in headless serial mode
  -h, --help    Show this help
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --build-only) export QEMU_RUN=0 ;;
    --headless)   export QEMU_HEADLESS=1 ;;
    -h|--help)    usage; exit 0 ;;
    *) echo "start.sh: unknown: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

# Step 1: Build base Alpine rootfs from alpenglow-os
"${ALPENGLOW_OS}/system/alpine/scripts/build-rootfs.sh"
"${ALPENGLOW_OS}/system/alpine/scripts/fetch-qemu-kernel.sh" "${ALPENGLOW_OS}/build/alpine/qemu"

# Step 2: Layer soliloquy desktop into the rootfs
ROOTFS="${ALPENGLOW_OS}/build/alpine/rootfs"
OVERLAY="${ROOT_DIR}/image-overlay"

mkdir -p "${ROOTFS}/usr/local/bin"

# Build and deploy soliloquy-daemon binary for ${QEMU_ARCH}
SOLD_BIN="${ROOT_DIR}/target/${QEMU_ARCH}-linux-musl/release/soliloquy-daemon"
if [ ! -f "${SOLD_BIN}" ]; then
  echo "  building soliloquy-daemon for ${QEMU_ARCH}..."
  BUILD_DIR="/tmp/sold-${QEMU_ARCH}-build"
  mkdir -p "${BUILD_DIR}/src"
  cp -r "${ROOT_DIR}/src/desktop/src/"* "${BUILD_DIR}/src/"
  cat > "${BUILD_DIR}/Cargo.toml" << EOF
[package]
name = "soliloquy-daemon"
version = "0.1.0"
edition = "2021"
[dependencies]
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
axum = { version = "0.8", features = ["ws", "json"] }
tower-http = { version = "0.6", features = ["cors", "fs"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
anyhow = "1.0"
log = "0.4"
env_logger = "0.11"
whoami = "1"
uuid = { version = "1.11", features = ["v4", "serde"] }
EOF
  PLATFORM="linux/amd64"
  [ "${QEMU_ARCH}" = "aarch64" ] && PLATFORM="linux/arm64"
  docker run --rm --platform="${PLATFORM}" -v "${BUILD_DIR}:/work" -w /work alpine:3.21 sh -c '
    apk add --no-cache curl gcc musl-dev 2>/dev/null
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal 2>/dev/null
    source "$HOME/.cargo/env"
    cargo build --release 2>&1 | tail -1
    cp target/release/soliloquy-daemon /work/soliloquy-daemon
  ' >/dev/null 2>&1
  mkdir -p "$(dirname "${SOLD_BIN}")"
  cp "${BUILD_DIR}/soliloquy-daemon" "${SOLD_BIN}" 2>/dev/null || true
  rm -rf "${BUILD_DIR}"
fi
if [ -f "${SOLD_BIN}" ]; then
  cp "${SOLD_BIN}" "${ROOTFS}/usr/local/bin/sold"
  chmod +x "${ROOTFS}/usr/local/bin/sold"
  echo "  sold: $(ls -lh "${ROOTFS}/usr/local/bin/sold" | awk '{print $5}')"
fi

# Strip bloat from rootfs (LLVM 154M, gallium 83M, fonts, X11, etc.)
rm -rf "${ROOTFS}/usr/lib/libLLVM.so.19.1" "${ROOTFS}/usr/lib/gallium-pipe" "${ROOTFS}/usr/lib/libgallium-24.2.8.so" 2>/dev/null || true
rm -rf "${ROOTFS}/usr/lib/dri" "${ROOTFS}/usr/share/fonts" "${ROOTFS}/usr/share/X11" 2>/dev/null || true
rm -rf "${ROOTFS}/usr/lib/gstreamer-1.0" "${ROOTFS}/usr/lib/alsa-lib" 2>/dev/null || true
echo "  trimmed: LLVM, gallium, DRI, fonts, X11, gstreamer, alsa"

# Replace Alpine init with fast boot — starts sold directly, no OpenRC
cp "${OVERLAY}/init" "${ROOTFS}/init"
chmod +x "${ROOTFS}/init"

# Step 4: Build initramfs with soliloquy desktop layered in
QEMU_DIR="${ALPENGLOW_OS}/build/alpine/qemu"
mkdir -p "${QEMU_DIR}"
(
  cd "${ROOTFS}"
  find . -print | cpio -o -H newc 2>/dev/null | gzip -9 > "${QEMU_DIR}/rootfs.cpio.gz"
)
echo "Initramfs: $(ls -lh "${QEMU_DIR}/rootfs.cpio.gz" | awk '{print $5}')"

# Step 4: Launch QEMU
if [ "${QEMU_RUN}" != "1" ]; then
  echo "QEMU_RUN=0 set; build artifacts ready."
  exit 0
fi

KERNEL="${QEMU_DIR}/vmlinuz-virt"
INITRAMFS="${QEMU_DIR}/rootfs.cpio.gz"

if [ "${QEMU_ARCH}" = "aarch64" ]; then
  SERIAL="ttyAMA0"
else
  SERIAL="ttyS0"
fi
KERNEL_CMDLINE="quiet loglevel=3 console=tty0 console=${SERIAL} random.trust_cpu=on rdinit=/init alpenglow.ram_root=auto alpenglow.ram_root_min_mb=3072 alpenglow.root_fallback=/dev/vda alpenglow.root_fallback_fstype=ext4"

# Use hvf acceleration on macOS for aarch64 guests (native speed)
ACCEL="tcg"
MACHINE="virt"
case "$(uname -s)" in
  Darwin)
    QEMU_BIN="$(which qemu-system-${QEMU_ARCH})"
    # Sign QEMU for HVF entitlement if not already signed
    if ! codesign -d --entitlements - "${QEMU_BIN}" 2>/dev/null | grep -q hypervisor; then
      cat >/tmp/qemu-hvf-entitlements.plist <<'EOF2'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>com.apple.security.hypervisor</key><true/></dict></plist>
EOF2
      codesign -s - --entitlements /tmp/qemu-hvf-entitlements.plist -f "${QEMU_BIN}" 2>/dev/null || true
      rm -f /tmp/qemu-hvf-entitlements.plist
    fi
    ACCEL="hvf" ;;
esac

echo "Starting QEMU (${QEMU_ARCH}, accel=${ACCEL})..."
exec qemu-system-${QEMU_ARCH} \
  -machine "${MACHINE},accel=${ACCEL}" \
  -m 2048 -smp 4 \
  $( [ "${QEMU_HEADLESS}" = "1" ] && echo "-nographic" || echo "-display default" ) \
  -device virtio-gpu-pci \
  -device virtio-keyboard-pci -device virtio-mouse-pci \
  -object rng-random,filename=/dev/urandom,id=rng0 -device virtio-rng-pci,rng=rng0 \
  -device virtio-net-pci,netdev=n1 -netdev user,id=n1 \
  -kernel "${KERNEL}" -initrd "${INITRAMFS}" \
  -append "${KERNEL_CMDLINE}"
