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
SOLILOQUY_UID="770"
SOLILOQUY_GID="770"
SOLD_UID="771"
SOLD_GID="771"

if [ ! -d "${ROOTFS}" ]; then
  echo "rootfs directory not found: ${ROOTFS}" >&2
  exit 1
fi

ensure_group() {
  name="$1"
  gid="$2"
  if ! grep -q "^${name}:" "${ROOTFS}/etc/group" 2>/dev/null; then
    printf '%s:x:%s:\n' "${name}" "${gid}" >>"${ROOTFS}/etc/group"
  fi
}

ensure_user() {
  name="$1"
  uid="$2"
  gid="$3"
  home="$4"
  if ! grep -q "^${name}:" "${ROOTFS}/etc/passwd" 2>/dev/null; then
    printf '%s:x:%s:%s:%s:%s:/sbin/nologin\n' "${name}" "${uid}" "${gid}" "${name}" "${home}" >>"${ROOTFS}/etc/passwd"
  fi
}

ensure_shadow() {
  name="$1"
  if [ -f "${ROOTFS}/etc/shadow" ] && ! grep -q "^${name}:" "${ROOTFS}/etc/shadow" 2>/dev/null; then
    printf '%s:!::0:::::\n' "${name}" >>"${ROOTFS}/etc/shadow"
  fi
}

ensure_group "soliloquy" "${SOLILOQUY_GID}"
ensure_group "sold" "${SOLD_GID}"
ensure_user "soliloquy" "${SOLILOQUY_UID}" "${SOLILOQUY_GID}" "/var/lib/soliloquy"
ensure_user "sold" "${SOLD_UID}" "${SOLD_GID}" "/var/lib/soliloquy/system"
ensure_shadow "soliloquy"
ensure_shadow "sold"

mkdir -p "${ROOTFS}/etc/init.d" "${ROOTFS}/usr/local/bin"
mkdir -p "${ROOTFS}/etc/local.d"
mkdir -p "${ROOTFS}/etc/network"
mkdir -p "${ROOTFS}/etc/soliloquy/plugins"
mkdir -p "${ROOTFS}/etc/soliloquy/services"
mkdir -p \
  "${ROOTFS}/var/lib/soliloquy/browser/profiles" \
  "${ROOTFS}/var/lib/soliloquy/browser/cache" \
  "${ROOTFS}/var/lib/soliloquy/browser/downloads" \
  "${ROOTFS}/var/lib/soliloquy/browser/state" \
  "${ROOTFS}/var/lib/soliloquy/browser/logs" \
  "${ROOTFS}/var/lib/soliloquy/browser/terminal" \
  "${ROOTFS}/var/lib/soliloquy/system" \
  "${ROOTFS}/var/lib/soliloquy/system/plugins"

chmod 700 \
  "${ROOTFS}/var/lib/soliloquy/browser/profiles" \
  "${ROOTFS}/var/lib/soliloquy/browser/cache" \
  "${ROOTFS}/var/lib/soliloquy/browser/downloads" \
  "${ROOTFS}/var/lib/soliloquy/browser/state" \
  "${ROOTFS}/var/lib/soliloquy/browser/logs" \
  "${ROOTFS}/var/lib/soliloquy/browser/terminal" \
  "${ROOTFS}/var/lib/soliloquy/system" \
  "${ROOTFS}/var/lib/soliloquy/system/plugins"

mkdir -p "${ROOTFS}/tmp"
chmod 755 "${ROOTFS}/tmp"

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

cat > "${ROOTFS}/etc/rc.conf" <<'EOF'
rc_logger="NO"
rc_parallel="YES"
rc_quiet_openrc="YES"
EOF

cat > "${ROOTFS}/etc/network/interfaces" <<'EOF'
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
EOF

cat > "${ROOTFS}/etc/resolv.conf" <<'EOF'
nameserver 10.0.2.3
EOF

cat > "${ROOTFS}/etc/soliloquy/system.json" <<'EOF'
{
  "filesystem": {
    "immutable_root": true,
    "user_home_root": "/home",
    "user_writable_scope": "home-only",
    "tmp_policy": {
      "path": "/tmp",
      "mode": "system-only"
    }
  },
  "browser": {
    "profile_management": "system",
    "profiles_root": "/var/lib/soliloquy/browser/profiles",
    "cache_root": "/var/lib/soliloquy/browser/cache",
    "state_root": "/var/lib/soliloquy/browser/state",
    "logs_root": "/var/lib/soliloquy/browser/logs"
  },
  "plugins": [
    {
      "id": "remote-sync",
      "display_name": "Remote Sync",
      "kind": "optional-download",
      "enabled": false,
      "sync": {
        "files": false,
        "photos": false,
        "clipboard": false
      }
    }
  ]
}
EOF

cat > "${ROOTFS}/etc/soliloquy/plugins/remote-sync.json" <<'EOF'
{
  "id": "remote-sync",
  "display_name": "Remote Sync",
  "kind": "optional-download",
  "entrypoint": "/var/lib/soliloquy/system/plugins/remote-sync",
  "capabilities": [
    "profile-sync",
    "encrypted-relay",
    "cross-device-sync"
  ],
  "sync_features": {
    "files": false,
    "photos": false,
    "clipboard": false
  }
}
EOF

cat > "${ROOTFS}/etc/soliloquy/services.json" <<'EOF'
{
  "services": [
    {
      "id": "sold",
      "display_name": "Soliloquy Local Server",
      "run_as": "sold",
      "restart": "always",
      "dependencies": ["networking"],
      "state_paths": [
        "/var/lib/soliloquy/system",
        "/var/log/soliloquy"
      ]
    },
    {
      "id": "sol-session",
      "display_name": "Soliloquy Session",
      "run_as": "root",
      "restart": "always",
      "dependencies": ["sold", "seatd"],
      "state_paths": [
        "/run/user/0",
        "/var/lib/soliloquy/browser"
      ]
    },
    {
      "id": "remote-sync",
      "display_name": "Remote Sync Plugin",
      "run_as": "sold",
      "restart": "on-failure",
      "dependencies": ["sold"],
      "optional": true,
      "state_paths": [
        "/var/lib/soliloquy/system/plugins"
      ]
    }
  ]
}
EOF

cat > "${ROOTFS}/var/lib/soliloquy/system/plugin-state.json" <<'EOF'
{
  "plugins": [
    {
      "id": "remote-sync",
      "display_name": "Remote Sync",
      "kind": "optional-download",
      "enabled": false,
      "sync": {
        "files": false,
        "photos": false,
        "clipboard": false
      }
    }
  ]
}
EOF

cat > "${ROOTFS}/etc/local.d/soliloquy-firstboot.start" <<'EOF'
#!/bin/sh
set -eu

chown -R soliloquy:soliloquy /var/lib/soliloquy/browser >/dev/null 2>&1 || true
chown -R sold:sold /var/lib/soliloquy/system >/dev/null 2>&1 || true

if command -v rc-update >/dev/null 2>&1; then
  # Keep the service graph minimal for browser appliance mode.
  for svc in acpid avahi-daemon bluetooth cron cupsd hwdrivers localmount machine-id nftables syslog wpa_supplicant; do
    rc-update del "${svc}" default >/dev/null 2>&1 || true
  done
  rc-update add local default >/dev/null 2>&1 || true
  rc-update add networking default >/dev/null 2>&1 || true
  rc-update add seatd default >/dev/null 2>&1 || true
fi
EOF

chmod +x "${ROOTFS}/etc/local.d/soliloquy-firstboot.start"

# Make default runlevel explicit and minimal.
mkdir -p "${ROOTFS}/etc/runlevels/default"
find "${ROOTFS}/etc/runlevels/default" -mindepth 1 -maxdepth 1 -exec rm -f {} +
for svc in local networking seatd sold; do
  if [ -f "${ROOTFS}/etc/init.d/${svc}" ]; then
    ln -sf "/etc/init.d/${svc}" "${ROOTFS}/etc/runlevels/default/${svc}"
  fi
done

echo "Configured Soliloquy Alpine rootfs at ${ROOTFS}"
