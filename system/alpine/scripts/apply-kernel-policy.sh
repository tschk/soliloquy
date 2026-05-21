#!/bin/sh
set -eu

SOLILOQUY_KERNEL_POLICY_FILE="${SOLILOQUY_KERNEL_POLICY_FILE:-/etc/soliloquy/kernel-policy.json}"
SOLILOQUY_RUNTIME_STATE_ENV="${SOLILOQUY_RUNTIME_STATE_ENV:-/run/soliloquy/runtime-state.env}"

if [ -x /usr/local/bin/sol-kernelctl ]; then
  exec /usr/local/bin/sol-kernelctl \
    --policy "${SOLILOQUY_KERNEL_POLICY_FILE}" \
    --runtime-state "${SOLILOQUY_RUNTIME_STATE_ENV}"
fi

mkdir -p "$(dirname "${SOLILOQUY_RUNTIME_STATE_ENV}")"

record_runtime_state() {
  key="$1"
  value="$2"
  tmp="${SOLILOQUY_RUNTIME_STATE_ENV}.$$"
  if [ -f "${SOLILOQUY_RUNTIME_STATE_ENV}" ]; then
    grep -v "^${key}=" "${SOLILOQUY_RUNTIME_STATE_ENV}" >"${tmp}" || true
  else
    : >"${tmp}"
  fi
  printf '%s=%s\n' "${key}" "${value}" >>"${tmp}"
  mv "${tmp}" "${SOLILOQUY_RUNTIME_STATE_ENV}"
}

apply_sysctl() {
  key="$1"
  value="$2"
  if command -v sysctl >/dev/null 2>&1; then
    sysctl -q -w "${key}=${value}" >/dev/null 2>&1 || true
  fi
}

load_kernel_module() {
  module="$1"
  if command -v modprobe >/dev/null 2>&1; then
    modprobe "${module}" >/dev/null 2>&1 || true
  fi
}

enable_controller() {
  controller="$1"
  if [ -f /sys/fs/cgroup/cgroup.subtree_control ]; then
    printf '+%s\n' "${controller}" > /sys/fs/cgroup/cgroup.subtree_control 2>/dev/null || true
  fi
}

set_cgroup_value() {
  group="$1"
  file="$2"
  value="$3"
  path="/sys/fs/cgroup/soliloquy/${group}/${file}"
  if [ -f "${path}" ]; then
    printf '%s\n' "${value}" >"${path}" 2>/dev/null || true
  fi
}

prepare_group() {
  group="$1"
  if [ -f /sys/fs/cgroup/cgroup.controllers ]; then
    mkdir -p "/sys/fs/cgroup/soliloquy/${group}" 2>/dev/null || true
  fi
}

if [ -f "${SOLILOQUY_KERNEL_POLICY_FILE}" ]; then
  record_runtime_state SOLILOQUY_KERNEL_POLICY_FILE "${SOLILOQUY_KERNEL_POLICY_FILE}"
fi

load_kernel_module virtio_pci
load_kernel_module virtio_net
load_kernel_module virtio_rng
load_kernel_module virtio_gpu

apply_sysctl net.core.somaxconn 4096
apply_sysctl net.core.default_qdisc fq
apply_sysctl net.ipv4.tcp_congestion_control bbr
apply_sysctl net.ipv4.tcp_fastopen 3
apply_sysctl net.ipv4.tcp_fin_timeout 15
apply_sysctl net.ipv4.tcp_keepalive_time 60
apply_sysctl net.ipv4.tcp_mtu_probing 1
apply_sysctl net.ipv4.tcp_syncookies 1
apply_sysctl net.ipv4.tcp_tw_reuse 1
apply_sysctl net.ipv4.ip_forward 0
apply_sysctl net.ipv4.conf.all.accept_redirects 0
apply_sysctl net.ipv4.conf.all.rp_filter 2
apply_sysctl net.ipv4.conf.all.send_redirects 0
apply_sysctl vm.swappiness 20
apply_sysctl vm.vfs_cache_pressure 50
apply_sysctl kernel.unprivileged_bpf_disabled 1

if [ -f /sys/fs/cgroup/cgroup.controllers ]; then
  mkdir -p /sys/fs/cgroup/soliloquy 2>/dev/null || true
  for controller in cpu io memory pids; do
    enable_controller "${controller}"
  done

  prepare_group system
  prepare_group browser
  prepare_group renderer

  set_cgroup_value system cpu.weight 100
  set_cgroup_value system io.weight 100
  set_cgroup_value system memory.high 256M
  set_cgroup_value system pids.max 128

  set_cgroup_value browser cpu.weight 300
  set_cgroup_value browser io.weight 300
  set_cgroup_value browser memory.high 768M
  set_cgroup_value browser pids.max 256

  set_cgroup_value renderer cpu.weight 800
  set_cgroup_value renderer io.weight 800
  set_cgroup_value renderer memory.high 1536M
  set_cgroup_value renderer memory.max 2304M
  set_cgroup_value renderer pids.max 512

  record_runtime_state SOLILOQUY_KERNEL_POLICY_CGROUPS active
else
  record_runtime_state SOLILOQUY_KERNEL_POLICY_CGROUPS unavailable
fi

record_runtime_state SOLILOQUY_KERNEL_POLICY_PROFILE internet-appliance
