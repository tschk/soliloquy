#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/../../.." && pwd)"
QEMU_ARCH="${QEMU_ARCH:-x86_64}"
OUT_DIR="${ROOT_DIR}/build/alpine/artifacts/linux-${QEMU_ARCH}"
SERVO_SRC_BIN="${ROOT_DIR}/third_party/servo/target/release/servoshell"
SERVO_RELEASE_TAG="${SERVO_RELEASE_TAG:-v0.0.6}"

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required tool: $1" >&2
    exit 1
  fi
}

file_matches_arch() {
  bin_path="$1"
  file_info="$(file "${bin_path}")"
  case "${QEMU_ARCH}" in
    x86_64)
      case "${file_info}" in
        *"ELF 64-bit"*x86-64*) return 0 ;;
      esac
      ;;
    aarch64|arm64)
      case "${file_info}" in
        *"ELF 64-bit"*aarch64*) return 0 ;;
      esac
      ;;
  esac
  return 1
}

build_sold_linux() {
  require_tool docker

  case "${QEMU_ARCH}" in
    x86_64)
      docker_platform="linux/amd64"
      ;;
    aarch64|arm64)
      docker_platform="linux/arm64"
      ;;
    *)
      echo "unsupported QEMU_ARCH: ${QEMU_ARCH}" >&2
      exit 1
      ;;
  esac

  echo "Building Linux sold for ${QEMU_ARCH}..." >&2
  docker run --rm \
    --platform "${docker_platform}" \
    -v "${ROOT_DIR}:/work" \
    -w /work \
    rust:1.85-alpine sh -lc "
      set -eu
      export PATH=/usr/local/cargo/bin:\$PATH
      apk add --no-cache musl-dev
      cargo build --release -p sold
      cp target/release/sold build/alpine/artifacts/linux-${QEMU_ARCH}/sold
    "
}

fetch_servo_release_linux() {
  case "${QEMU_ARCH}" in
    x86_64) asset_name="servo-x86_64-linux-gnu.tar.gz" ;;
    *)
      echo "no default Servo release asset configured for QEMU_ARCH=${QEMU_ARCH}" >&2
      echo "set SERVO_BIN_LINUX to a Linux ELF servoshell/servo binary path" >&2
      exit 1
      ;;
  esac

  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "${tmp_dir}"' EXIT INT TERM
  archive_path="${tmp_dir}/${asset_name}"
  url="https://github.com/servo/servo/releases/download/${SERVO_RELEASE_TAG}/${asset_name}"

  echo "Fetching Servo Linux release ${SERVO_RELEASE_TAG} (${asset_name})..." >&2
  curl -fsSL "${url}" -o "${archive_path}"
  tar -xzf "${archive_path}" -C "${tmp_dir}"

  if [ ! -f "${tmp_dir}/servo/servo" ]; then
    echo "servo release archive missing expected binary at servo/servo" >&2
    exit 1
  fi

  cp "${tmp_dir}/servo/servo" "${OUT_DIR}/servo"
}

mkdir -p "${OUT_DIR}"

if [ -f "${OUT_DIR}/sold" ] && file_matches_arch "${OUT_DIR}/sold"; then
  echo "Reusing Linux sold binary: ${OUT_DIR}/sold" >&2
else
  build_sold_linux
fi

if [ -n "${SERVO_BIN_LINUX:-}" ]; then
  if [ ! -f "${SERVO_BIN_LINUX}" ]; then
    echo "SERVO_BIN_LINUX does not exist: ${SERVO_BIN_LINUX}" >&2
    exit 1
  fi
  if ! file_matches_arch "${SERVO_BIN_LINUX}"; then
    echo "SERVO_BIN_LINUX is not Linux ELF for ${QEMU_ARCH}: ${SERVO_BIN_LINUX}" >&2
    echo "detected: $(file "${SERVO_BIN_LINUX}")" >&2
    exit 1
  fi
  cp "${SERVO_BIN_LINUX}" "${OUT_DIR}/servo"
elif [ -f "${SERVO_SRC_BIN}" ] && file_matches_arch "${SERVO_SRC_BIN}"; then
  echo "Using in-tree Linux Servo binary: ${SERVO_SRC_BIN}" >&2
  cp "${SERVO_SRC_BIN}" "${OUT_DIR}/servo"
else
  fetch_servo_release_linux
fi

chmod +x "${OUT_DIR}/servo" "${OUT_DIR}/sold"
echo "${OUT_DIR}"
