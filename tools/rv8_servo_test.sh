#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
servo_dir="${SERVO_DIR:-"$repo_root/third_party/servo"}"

if [[ "$(uname -s)" == "Darwin" ]]; then
  sdkroot="${SDKROOT:-}"
  if [[ -z "$sdkroot" ]]; then
    sdkroot="$(xcrun --sdk macosx --show-sdk-path)"
  fi

  export SDKROOT="$sdkroot"
  export BINDGEN_EXTRA_CLANG_ARGS="${BINDGEN_EXTRA_CLANG_ARGS:--isysroot $sdkroot}"
  export BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_darwin="${BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_darwin:--isysroot $sdkroot}"
fi

cd "$servo_dir"

case "${1:-bridge}" in
  bridge)
    cargo test -p servo soliloquy_javascript --lib
    cargo test -p servo javascript_evaluator --lib
    ;;
  v8)
    cargo test -p servo soliloquy_javascript --lib --features soliloquy_v8
    ;;
  cargo)
    exec "$@"
    ;;
  *)
    exec cargo test -p servo "$@"
    ;;
esac
