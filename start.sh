#!/bin/sh
# Build Soliloquy desktop atop Alpenglow and boot in QEMU.
# Layers soliloquy-daemon (sold) into alpenglow base, replacing alpenglowed.
set -eu

ROOT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
ALPENGLOW_DIR="$(CDPATH='' cd -- "${ROOT_DIR}/../alpenglow" && pwd)"
QEMU_RUN="${QEMU_RUN:-1}"
QEMU_HEADLESS="${QEMU_HEADLESS:-0}"
QEMU_ARCH="${QEMU_ARCH:-x86_64}"
BUILD_PROFILE="${BUILD_PROFILE:-desktop}"

usage() {
  cat <<'USAGE'
Usage: ./start.sh [options]

Build the Soliloquy desktop atop Alpenglow and boot it in QEMU.

Options:
  --build-only  Prepare artifacts, skip VM launch
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

# Build sold (soliloquy-daemon) for target
SOLD_BIN="${ROOT_DIR}/target/release/soliloquy-daemon"
echo "→ Building soliloquy-daemon (sold)..."
cargo build -p soliloquy-daemon --release 2>&1 | tail -3
[ -f "${SOLD_BIN}" ] || { echo "ERROR: sold build failed"; exit 1; }
echo "  sold: $(ls -lh "${SOLD_BIN}" | awk '{print $5}')"

# Build alpenglow base via boot-native.sh (build-only mode)
echo "→ Building Alpenglow base (${BUILD_PROFILE}, ${QEMU_ARCH})..."
ALPENGLOW_OUT="${ALPENGLOW_DIR}/build/native"
ALPENGLOW_ROOTFS="${ALPENGLOW_OUT}/rootfs"

# Run alpenglow's boot-native in build-only mode if artifacts missing
if [ ! -f "${ALPENGLOW_OUT}/vmlinuz" ] || [ ! -d "${ALPENGLOW_ROOTFS}" ]; then
  (cd "${ALPENGLOW_DIR}" && \
   BUILD_PROFILE="${BUILD_PROFILE}" \
   GRAPHICAL=1 \
   QEMU_RUN=0 \
   sh scripts/boot-native.sh) 2>&1 | tail -10
fi

if [ ! -d "${ALPENGLOW_ROOTFS}" ]; then
  echo "ERROR: alpenglow rootfs not built at ${ALPENGLOW_ROOTFS}" >&2
  exit 1
fi

# Layer sold into rootfs
echo "→ Layering soliloquy into rootfs..."
cp "${SOLD_BIN}" "${ALPENGLOW_ROOTFS}/usr/local/bin/sold"
chmod 755 "${ALPENGLOW_ROOTFS}/usr/local/bin/sold"

# Copy soliloquy session start script
cp "${ROOT_DIR}/build/alpenglow/scripts/soliloquy-session-start" "${ALPENGLOW_ROOTFS}/usr/local/bin/"
chmod 755 "${ALPENGLOW_ROOTFS}/usr/local/bin/soliloquy-session-start"

# Replace alpenglowed dinit service with sold
cp "${ROOT_DIR}/build/alpenglow/dinit/sold" "${ALPENGLOW_ROOTFS}/etc/dinit.d/alpenglowed"
cp "${ROOT_DIR}/build/alpenglow/dinit/soliloquy-session" "${ALPENGLOW_ROOTFS}/etc/dinit.d/"

# Install soliloquy desktop UI bundle if built
UI_BUNDLE="${ROOT_DIR}/ui/desktop/build"
if [ -d "${UI_BUNDLE}" ]; then
  mkdir -p "${ALPENGLOW_ROOTFS}/usr/share/soliloquy/ui"
  cp -R "${UI_BUNDLE}/." "${ALPENGLOW_ROOTFS}/usr/share/soliloquy/ui/"
  echo "  UI bundle: $(du -sh "${ALPENGLOW_ROOTFS}/usr/share/soliloquy/ui" | cut -f1)"
fi

# Rebuild initramfs with sold layered in
echo "→ Rebuilding initramfs..."
INITRAMFS="${ALPENGLOW_OUT}/initramfs.cpio.zst"
(cd "${ALPENGLOW_ROOTFS}" && find . -print | cpio -o -H newc 2>/dev/null | zstd -6 -T0 > "${INITRAMFS}")
echo "  initramfs: ${INITRAMFS} ($(du -sh "${INITRAMFS}" | cut -f1))"

KERNEL="${ALPENGLOW_OUT}/vmlinuz"

if [ "${QEMU_RUN}" != "1" ]; then
  echo "QEMU_RUN=0 set; build artifacts ready."
  echo "  kernel:    ${KERNEL}"
  echo "  initramfs: ${INITRAMFS}"
  exit 0
fi

# Boot QEMU
require_cmd() { command -v "$1" >/dev/null 2>&1 || { echo "missing: $1"; exit 1; }; }
require_cmd qemu-system-${QEMU_ARCH}

echo "→ Booting Soliloquy desktop in QEMU..."
echo "  kernel:    ${KERNEL}"
echo "  initramfs: ${INITRAMFS}"
echo "  (Ctrl-A X to quit)"
echo ""

SERIAL="ttyS0"
[ "${QEMU_ARCH}" = "aarch64" ] && SERIAL="ttyAMA0"

KERNEL_CMDLINE="quiet console=ttyS0 console=tty0 init=/init"

# Auto-detect acceleration
ACCEL="tcg"
case "$(uname -s)" in
  Darwin)
    if timeout 2 qemu-system-${QEMU_ARCH} -machine virt,accel=hvf -M none </dev/null >/dev/null 2>&1; then
      ACCEL="hvf"
    fi
    ;;
  Linux)
    [ -c /dev/kvm ] && [ -r /dev/kvm ] && [ -w /dev/kvm ] && ACCEL="kvm"
    ;;
esac

MACHINE="q35"
[ "${QEMU_ARCH}" = "aarch64" ] && MACHINE="virt"

if [ "${QEMU_HEADLESS}" = "1" ]; then
  exec qemu-system-${QEMU_ARCH} \
    -machine "${MACHINE},accel=${ACCEL}" -m 4096 -smp 4 \
    -nographic -no-reboot \
    -device virtio-gpu-pci \
    -device virtio-keyboard-pci -device virtio-mouse-pci \
    -object rng-random,filename=/dev/urandom,id=rng0 -device virtio-rng-pci,rng=rng0 \
    -device virtio-net-pci,netdev=n1 -netdev user,id=n1 \
    -kernel "${KERNEL}" -initrd "${INITRAMFS}" \
    -append "${KERNEL_CMDLINE}"
else
  # Pick display backend
  QEMU_DISPLAY=""
  for backend in gtk sdl cocoa; do
    if timeout 2 qemu-system-${QEMU_ARCH} -display ${backend},show-cursor=off -M none </dev/null >/dev/null 2>&1; then
      QEMU_DISPLAY="${backend}"
      break
    fi
  done
  QEMU_DISPLAY="${QEMU_DISPLAY:-gtk}"

  exec qemu-system-${QEMU_ARCH} \
    -machine "${MACHINE},accel=${ACCEL}" -m 4096 -smp 4 -no-reboot \
    -display "${QEMU_DISPLAY}" \
    -device virtio-gpu-pci \
    -device virtio-keyboard-pci -device virtio-mouse-pci \
    -object rng-random,filename=/dev/urandom,id=rng0 -device virtio-rng-pci,rng=rng0 \
    -device virtio-net-pci,netdev=n1 -netdev user,id=n1 \
    -kernel "${KERNEL}" -initrd "${INITRAMFS}" \
    -append "${KERNEL_CMDLINE}"
fi
