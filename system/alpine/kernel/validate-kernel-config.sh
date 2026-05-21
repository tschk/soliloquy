#!/bin/sh
set -eu

CONFIG_FILE="${1:-$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/soliloquy-internet-appliance.config}"

if [ ! -f "${CONFIG_FILE}" ]; then
  printf 'validate-kernel-config: missing config: %s\n' "${CONFIG_FILE}" >&2
  exit 1
fi

failures=0

option_value() {
  option="$1"
  awk -v option="${option}" '
    $0 == option "=y" { value = "y" }
    $0 == option "=m" { value = "m" }
    $0 == option "=n" { value = "n" }
    $0 ~ "^" option "=" { value = substr($0, length(option) + 2) }
    $0 == "# " option " is not set" { value = "unset" }
    END {
      if (value != "") {
        print value
      }
    }
  ' "${CONFIG_FILE}"
}

require_enabled() {
  option="$1"
  value="$(option_value "${option}")"
  case "${value}" in
    y|m)
      return 0
      ;;
  esac
  printf 'validate-kernel-config: %s must be enabled, found %s\n' "${option}" "${value:-missing}" >&2
  failures=$((failures + 1))
}

require_builtin() {
  option="$1"
  value="$(option_value "${option}")"
  if [ "${value}" = "y" ]; then
    return 0
  fi
  printf 'validate-kernel-config: %s must be built in, found %s\n' "${option}" "${value:-missing}" >&2
  failures=$((failures + 1))
}

require_disabled() {
  option="$1"
  value="$(option_value "${option}")"
  case "${value}" in
    unset|n)
      return 0
      ;;
  esac
  printf 'validate-kernel-config: %s must be disabled, found %s\n' "${option}" "${value:-missing}" >&2
  failures=$((failures + 1))
}

require_value() {
  option="$1"
  expected="$2"
  value="$(option_value "${option}")"
  if [ "${value}" = "${expected}" ]; then
    return 0
  fi
  printf 'validate-kernel-config: %s must be %s, found %s\n' "${option}" "${expected}" "${value:-missing}" >&2
  failures=$((failures + 1))
}

for option in \
  CONFIG_CGROUPS \
  CONFIG_CGROUP_BPF \
  CONFIG_CGROUP_PIDS \
  CONFIG_CPUSETS \
  CONFIG_MEMCG \
  CONFIG_BLK_CGROUP \
  CONFIG_NAMESPACES \
  CONFIG_SECCOMP \
  CONFIG_SECCOMP_FILTER \
  CONFIG_SECURITY_LANDLOCK \
  CONFIG_NET_SCH_FQ \
  CONFIG_TCP_CONG_BBR \
  CONFIG_ZRAM \
  CONFIG_ZSMALLOC \
  CONFIG_SWAP \
  CONFIG_VIRTIO \
  CONFIG_VIRTIO_PCI \
  CONFIG_VIRTIO_BLK \
  CONFIG_VIRTIO_NET \
  CONFIG_VIRTIO_CONSOLE \
  CONFIG_HW_RANDOM_VIRTIO \
  CONFIG_DRM \
  CONFIG_DRM_KMS_HELPER \
  CONFIG_DRM_FBDEV_EMULATION \
  CONFIG_DRM_VIRTIO_GPU \
  CONFIG_DRM_SIMPLEDRM \
  CONFIG_EXT4_FS \
  CONFIG_SQUASHFS \
  CONFIG_OVERLAY_FS \
  CONFIG_TMPFS \
  CONFIG_DEVTMPFS \
  CONFIG_DEVTMPFS_MOUNT
do
  require_enabled "${option}"
done

for option in \
  CONFIG_CGROUPS \
  CONFIG_SECCOMP \
  CONFIG_SECURITY_LANDLOCK \
  CONFIG_VIRTIO \
  CONFIG_VIRTIO_PCI \
  CONFIG_VIRTIO_BLK \
  CONFIG_VIRTIO_NET \
  CONFIG_DRM \
  CONFIG_DRM_VIRTIO_GPU \
  CONFIG_ZRAM \
  CONFIG_EXT4_FS \
  CONFIG_TMPFS \
  CONFIG_DEVTMPFS
do
  require_builtin "${option}"
done

require_value CONFIG_DEFAULT_TCP_CONG '"bbr"'

for option in \
  CONFIG_ACPI \
  CONFIG_ANDROID \
  CONFIG_APPLETALK \
  CONFIG_ATM \
  CONFIG_BCACHEFS_FS \
  CONFIG_BFS_FS \
  CONFIG_BLUETOOTH \
  CONFIG_BTRFS_FS \
  CONFIG_CAN \
  CONFIG_CEPH_FS \
  CONFIG_CIFS \
  CONFIG_DLM \
  CONFIG_DRM_NOUVEAU \
  CONFIG_DRM_RADEON \
  CONFIG_DRM_XE \
  CONFIG_FIREWIRE \
  CONFIG_GFS2_FS \
  CONFIG_HAMRADIO \
  CONFIG_HFS_FS \
  CONFIG_HFSPLUS_FS \
  CONFIG_INFINIBAND \
  CONFIG_IP_DCCP \
  CONFIG_IP_SCTP \
  CONFIG_IPX \
  CONFIG_ISDN \
  CONFIG_JFS_FS \
  CONFIG_NFC \
  CONFIG_NFS_FS \
  CONFIG_OCFS2_FS \
  CONFIG_PPP \
  CONFIG_REISERFS_FS \
  CONFIG_SCSI \
  CONFIG_SMB_SERVER \
  CONFIG_SOUND \
  CONFIG_USB_GADGET \
  CONFIG_USB_STORAGE \
  CONFIG_WIMAX \
  CONFIG_WIRELESS_EXT \
  CONFIG_X25
do
  require_disabled "${option}"
done

if [ "${failures}" -ne 0 ]; then
  printf 'validate-kernel-config: failed with %s error(s)\n' "${failures}" >&2
  exit 1
fi

printf 'validate-kernel-config: ok %s\n' "${CONFIG_FILE}"
