#!/bin/sh
set -eu

REPO_ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
cd "${REPO_ROOT}"

fail() {
  printf 'ci-os-appliance: %s\n' "$1" >&2
  exit 1
}

assert_file() {
  [ -f "$1" ] || fail "missing file: $1"
}

assert_executable() {
  [ -x "$1" ] || fail "missing executable: $1"
}

assert_contains() {
  file="$1"
  pattern="$2"
  if ! grep -Eq "${pattern}" "${file}"; then
    fail "${file} does not match ${pattern}"
  fi
}

assert_not_contains() {
  file="$1"
  pattern="$2"
  if grep -Eq "${pattern}" "${file}"; then
    fail "${file} unexpectedly matches ${pattern}"
  fi
}

assert_runlevel_service() {
  rootfs="$1"
  service="$2"
  link="${rootfs}/etc/runlevels/default/${service}"
  [ -L "${link}" ] || fail "missing default runlevel service: ${service}"
  [ "$(readlink "${link}")" = "/etc/init.d/${service}" ] || fail "bad runlevel target for ${service}"
}

test -L CLAUDE.md || fail "CLAUDE.md must be a symlink"
[ "$(readlink CLAUDE.md)" = "AGENTS.md" ] || fail "CLAUDE.md must point to AGENTS.md"
[ ! -e src/rv8 ] || fail "src/rv8 must stay deleted"

for path in \
  system/alpine/scripts/configure-rootfs.sh \
  system/alpine/scripts/apply-kernel-policy.sh \
  system/alpine/scripts/apply-zram-policy.sh \
  system/alpine/scripts/audit-package-budget.sh \
  system/alpine/scripts/build-native-policy-modules.sh \
  system/alpine/scripts/sol-session-start \
  system/alpine/scripts/sol-servo-wrapper \
  system/alpine/scripts/stage-soliloquy-artifacts.sh \
  system/alpine/openrc/sold \
  system/alpine/openrc/sol-session \
  system/alpine/openrc/sol-kernel-policy \
  system/alpine/openrc/sol-zram \
  system/alpine/openrc/seatd
do
  assert_file "${path}"
  sh -n "${path}"
done

assert_file system/alpine/kernel-policy.json
assert_file system/native/kernel-policy-v/policy.v
assert_file system/native/kernel-policy-v/v.mod
assert_contains system/alpine/kernel-policy.json '"profile": "internet-appliance"'
assert_contains system/alpine/kernel-policy.json '"net.core.somaxconn"'
assert_contains system/alpine/scripts/build-native-policy-modules.sh '../equilibrium'
assert_contains system/alpine/scripts/build-native-policy-modules.sh 'libsoliloquy_native_policy_v.so'
assert_contains system/alpine/scripts/build-native-policy-modules.sh 'native policy userland module'
assert_not_contains system/alpine/scripts/build-native-policy-modules.sh 'Built V kernel'
assert_contains system/alpine/scripts/stage-soliloquy-artifacts.sh 'NATIVE_POLICY_REQUIRED'
assert_contains system/alpine/scripts/stage-soliloquy-artifacts.sh '/usr/local/lib/soliloquy/native-policy'
assert_contains system/native/kernel-policy-v/policy.v 'sol_renderer_cpu_weight'
assert_contains system/native/kernel-policy-v/v.mod "license: 'MPL-2.0'"
assert_contains system/alpine/openrc/sol-session '^respawn=YES$'
assert_contains system/alpine/openrc/sol-session 'need sold seatd'
assert_contains system/alpine/openrc/sold 'need localmount networking'
assert_contains system/alpine/openrc/sol-kernel-policy 'apply-kernel-policy.sh'
assert_contains system/alpine/openrc/sol-zram 'apply-zram-policy.sh'
assert_contains system/alpine/openrc/seatd '^command_background="yes"$'
assert_file system/alpine/rootfs-overlay/etc/modprobe.d/soliloquy-browser-appliance.conf
assert_file system/alpine/rootfs-overlay/etc/modules-load.d/soliloquy-browser-appliance.conf
assert_contains system/alpine/rootfs-overlay/etc/modprobe.d/soliloquy-browser-appliance.conf '^blacklist bluetooth$'
assert_contains system/alpine/rootfs-overlay/etc/modprobe.d/soliloquy-browser-appliance.conf '^blacklist usb_storage$'
assert_contains system/alpine/rootfs-overlay/etc/modules-load.d/soliloquy-browser-appliance.conf '^zram$'
assert_contains system/alpine/rootfs-overlay/init 'mount -t cgroup2 cgroup2 /sys/fs/cgroup'
assert_contains system/alpine/rootfs-overlay/etc/inittab '^::wait:/sbin/openrc default$'
assert_not_contains system/alpine/rootfs-overlay/etc/inittab 'sol-session-start'

assert_contains system/alpine/scripts/apply-kernel-policy.sh 'load_kernel_module virtio_net'
assert_contains system/alpine/scripts/apply-kernel-policy.sh 'sol-kernelctl'
assert_contains system/alpine/scripts/apply-kernel-policy.sh 'cgroup.subtree_control'
assert_contains system/alpine/scripts/apply-kernel-policy.sh 'memory.high'
assert_contains system/alpine/scripts/apply-kernel-policy.sh 'io.weight'
assert_contains system/alpine/scripts/apply-zram-policy.sh 'modprobe zram'
assert_contains system/alpine/scripts/apply-zram-policy.sh 'swapon -p 100 /dev/zram0'
assert_contains system/alpine/scripts/audit-package-budget.sh 'SOLILOQUY_MAX_RUNTIME_PACKAGES'
assert_contains system/alpine/packages-v0.txt '^font-dejavu$'
assert_not_contains system/alpine/packages-v0.txt '^xwayland$|^gcompat$|^font-noto$'
assert_contains system/alpine/scripts/sol-session-start 'SOLILOQUY_RUNTIME_STATE_ENV'
assert_contains system/alpine/scripts/sol-session-start 'SOL_KERNEL_POLICY_REQUIRED'
assert_contains system/alpine/rootfs-overlay/etc/conf.d/sol-session '^SOL_KERNEL_POLICY_REQUIRED=1$'
assert_contains system/alpine/rootfs-overlay/etc/conf.d/sol-session '^SOL_SESSION_X11_FALLBACK=0$'
assert_contains system/alpine/scripts/sol-session-start 'WLR_XWAYLAND'
assert_contains system/alpine/scripts/sol-session-start 'sol-kernelctl attach --group'
assert_contains system/alpine/scripts/sol-session-start 'attach_to_cgroup browser'
assert_contains system/alpine/scripts/sol-servo-wrapper 'attach_to_cgroup renderer'
assert_contains system/alpine/scripts/sol-servo-wrapper 'sol-kernelctl attach --group'
assert_contains system/alpine/openrc/sold 'attach_to_cgroup system'
assert_contains system/alpine/openrc/sold 'sol-kernelctl attach --group'
assert_contains system/alpine/services.json '"id": "networking"'
assert_contains system/alpine/services.json '"id": "sol-zram"'
assert_contains system/alpine/services.json '"id": "sold"'
assert_contains system/alpine/services.json '"id": "sol-session"'
assert_not_contains system/alpine/scripts/stage-soliloquy-artifacts.sh 'src/rv8|release/rv8|cargo .*rv8'
assert_not_contains system/alpine/scripts/configure-rootfs.sh 'dev-signature-placeholder|fake|placeholder signature'

tmp_root="$(mktemp -d)"
trap 'rm -rf "${tmp_root}"' EXIT INT TERM
mkdir -p "${tmp_root}/etc/init.d" "${tmp_root}/etc/runlevels/default"
: >"${tmp_root}/etc/passwd"
: >"${tmp_root}/etc/group"
: >"${tmp_root}/etc/shadow"
for service in local networking; do
  printf '#!/sbin/openrc-run\n' >"${tmp_root}/etc/init.d/${service}"
done

system/alpine/scripts/configure-rootfs.sh "${tmp_root}" >/dev/null

for service in networking seatd sol-kernel-policy sol-zram sold sol-session; do
  assert_runlevel_service "${tmp_root}" "${service}"
done

assert_file "${tmp_root}/etc/soliloquy/services.json"
assert_file "${tmp_root}/etc/soliloquy/system.json"
assert_file "${tmp_root}/etc/soliloquy/kernel-policy.json"
assert_file "${tmp_root}/etc/soliloquy/package-manager.json"
assert_file "${tmp_root}/var/lib/soliloquy/system/plugin-installs.json"
assert_executable "${tmp_root}/etc/init.d/sol-kernel-policy"
assert_executable "${tmp_root}/etc/init.d/sol-zram"
assert_executable "${tmp_root}/usr/local/bin/apply-kernel-policy.sh"
assert_executable "${tmp_root}/usr/local/bin/apply-zram-policy.sh"
assert_executable "${tmp_root}/usr/local/bin/sol-session-start"
assert_executable "${tmp_root}/usr/local/bin/sol-servo-wrapper"
assert_not_contains "${tmp_root}/etc/inittab" 'sol-session-start'
assert_contains "${tmp_root}/etc/rc.conf" '^rc_parallel="YES"$'
assert_contains "${tmp_root}/etc/sysctl.d/99-soliloquy-internet-os.conf" '^net.core.somaxconn=4096$'
assert_contains "${tmp_root}/etc/modprobe.d/soliloquy-browser-appliance.conf" '^blacklist usb_storage$'
assert_contains "${tmp_root}/etc/modules-load.d/soliloquy-browser-appliance.conf" '^zram$'
assert_contains "${tmp_root}/var/lib/soliloquy/system/plugin-installs.json" '"plugins": \[\]'
assert_contains "${tmp_root}/etc/soliloquy/services.json" '"id": "seatd"'
assert_contains "${tmp_root}/etc/soliloquy/services.json" '"id": "sol-kernel-policy"'
assert_contains "${tmp_root}/etc/soliloquy/services.json" '"id": "sol-zram"'
assert_contains "${tmp_root}/etc/soliloquy/services.json" '"id": "sol-session"'
[ ! -e "${tmp_root}/etc/runlevels/default/local" ] || fail "local must not block browser appliance boot"

printf 'ci-os-appliance: ok\n'
