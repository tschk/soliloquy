#!/bin/sh
set -eu

ROOTFS="${1:-}"
if [ -z "${ROOTFS}" ]; then
  echo "usage: $0 <rootfs-dir>" >&2
  exit 1
fi

SCRIPT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
ALPINE_DIR="$(CDPATH='' cd -- "${SCRIPT_DIR}/.." && pwd)"
OVERLAY_DIR="${ALPINE_DIR}/rootfs-overlay"
OPENRC_DIR="${ALPINE_DIR}/openrc"
BIN_SRC="${ALPINE_DIR}/scripts"

if [ ! -d "${ROOTFS}" ]; then
  echo "rootfs directory not found: ${ROOTFS}" >&2
  exit 1
fi

mkdir -p "${ROOTFS}/etc/init.d" "${ROOTFS}/usr/local/bin"
mkdir -p "${ROOTFS}/etc/local.d"
mkdir -p \
  "${ROOTFS}/var/lib/soliloquy/browser/profile" \
  "${ROOTFS}/var/lib/soliloquy/browser/cache" \
  "${ROOTFS}/var/lib/soliloquy/browser/downloads" \
  "${ROOTFS}/var/lib/soliloquy/browser/state" \
  "${ROOTFS}/var/lib/soliloquy/browser/logs" \
  "${ROOTFS}/var/lib/soliloquy/browser/terminal"

chmod 700 \
  "${ROOTFS}/var/lib/soliloquy/browser/profile" \
  "${ROOTFS}/var/lib/soliloquy/browser/cache" \
  "${ROOTFS}/var/lib/soliloquy/browser/downloads" \
  "${ROOTFS}/var/lib/soliloquy/browser/state" \
  "${ROOTFS}/var/lib/soliloquy/browser/logs" \
  "${ROOTFS}/var/lib/soliloquy/browser/terminal"

cp -R "${OVERLAY_DIR}/." "${ROOTFS}/"
cp "${OPENRC_DIR}/sol-session" "${ROOTFS}/etc/init.d/sol-session"
if [ -f "${OPENRC_DIR}/sold" ]; then
  cp "${OPENRC_DIR}/sold" "${ROOTFS}/etc/init.d/sold"
fi
cp "${BIN_SRC}/sol-session-start" "${ROOTFS}/usr/local/bin/sol-session-start"
cp "${BIN_SRC}/sol-servo-wrapper" "${ROOTFS}/usr/local/bin/sol-servo-wrapper"

chmod +x \
  "${ROOTFS}/etc/init.d/sol-session" \
  "${ROOTFS}/etc/init.d/sold" \
  "${ROOTFS}/usr/local/bin/sol-session-start" \
  "${ROOTFS}/usr/local/bin/sol-servo-wrapper" \
  "${ROOTFS}/init"

if [ -f "${ALPINE_DIR}/packages-v0.txt" ]; then
  cp "${ALPINE_DIR}/packages-v0.txt" "${ROOTFS}/etc/apk/world.soliloquy"
fi
if [ -f "${ALPINE_DIR}/packages-v0-dev.txt" ]; then
  cp "${ALPINE_DIR}/packages-v0-dev.txt" "${ROOTFS}/etc/apk/world.soliloquy.dev"
fi

cat > "${ROOTFS}/etc/local.d/soliloquy-firstboot.start" <<'EOF'
#!/bin/sh
set -eu

if command -v rc-update >/dev/null 2>&1; then
  # Keep the service graph minimal for browser appliance mode.
  for svc in acpid avahi-daemon bluetooth cron cupsd hwdrivers localmount machine-id nftables networking syslog wpa_supplicant; do
    rc-update del "${svc}" default >/dev/null 2>&1 || true
  done
  rc-update add local default >/dev/null 2>&1 || true
  rc-update add seatd default >/dev/null 2>&1 || true
fi
EOF

chmod +x "${ROOTFS}/etc/local.d/soliloquy-firstboot.start"

# Make default runlevel explicit and minimal.
mkdir -p "${ROOTFS}/etc/runlevels/default"
find "${ROOTFS}/etc/runlevels/default" -mindepth 1 -maxdepth 1 -exec rm -f {} +
for svc in local seatd sold; do
  if [ -f "${ROOTFS}/etc/init.d/${svc}" ]; then
    ln -sf "/etc/init.d/${svc}" "${ROOTFS}/etc/runlevels/default/${svc}"
  fi
done

echo "Configured Soliloquy Alpine rootfs at ${ROOTFS}"
