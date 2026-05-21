#!/bin/sh
set -eu

REPO_ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
cd "${REPO_ROOT}"

fail() {
  printf 'ci-rust-core: %s\n' "$1" >&2
  exit 1
}

metadata="$(cargo metadata --no-deps --format-version 1)"
printf '%s\n' "${metadata}" | grep -q '"name":"rv8"' && fail "rv8 must not be a root workspace package"

cargo fmt --package sold -- --check
cargo test -p sold
cargo test -p soliloquy_browser_optimizations --lib
cargo test -p soliloquy-shell --lib

printf 'ci-rust-core: ok\n'
