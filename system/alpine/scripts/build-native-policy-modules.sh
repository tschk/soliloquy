#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/../../.." && pwd)"
EQUILIBRIUM_DIR="${EQUILIBRIUM_DIR:-${ROOT_DIR}/../equilibrium}"
NATIVE_SRC="${ROOT_DIR}/system/native/kernel-policy-v/policy.v"
OUT_DIR="${OUT_DIR:-${ROOT_DIR}/build/alpine/native}"

if [ ! -f "${NATIVE_SRC}" ]; then
  echo "missing V policy source: ${NATIVE_SRC}" >&2
  exit 1
fi

mkdir -p "${OUT_DIR}"

if [ -x "${EQUILIBRIUM_DIR}/target/release/eq" ]; then
  "${EQUILIBRIUM_DIR}/target/release/eq" check >/dev/null 2>&1 || true
elif [ -f "${EQUILIBRIUM_DIR}/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
  cargo run --manifest-path "${EQUILIBRIUM_DIR}/Cargo.toml" --features cli --bin eq -- check >/dev/null 2>&1 || true
fi

if command -v v >/dev/null 2>&1; then
  if v -shared -prod -o "${OUT_DIR}/libsol_policy_v.so" "${NATIVE_SRC}"; then
    echo "Built V kernel policy module: ${OUT_DIR}/libsol_policy_v.so"
  else
    echo "V compiler failed; skipped optional V kernel policy module" >&2
  fi
else
  echo "V compiler not found; skipped optional V kernel policy module" >&2
fi
