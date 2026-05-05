# CLAUDE.md - Soliloquy Project Guide

## What Is Soliloquy?

Soliloquy is an experimental browser appliance and web-native shell. The current repo is centered on a Linux/Alpine runtime, a Rust shell, Servo integration work, a V8 runtime layer, and local UI/services for a single-purpose browser environment.

The project is early-stage and not production-ready.

## Current Repo Shape

There are four active Rust packages in the root Cargo workspace:

- `soliloquy_browser_optimizations` (`src/`) - shared cache, memory, GPU, network, and V8 support code
- `soliloquy-shell` (`src/shell/`) - shell library plus `soliloquy_shell` and `soliloquy_shell_simple` binaries
- `rv8` (`src/rv8/`) - experimental browser engine/runtime crate
- `sold` (`sold/`) - local Axum service for bundled UI, files, and settings

There are also substantial adjacent areas in-tree, including:

- `system/alpine/` - Alpine appliance build, staging, and QEMU boot flow
- `ui/desktop/` - Svelte desktop UI
- `third_party/servo/` - in-tree Servo checkout used by the Alpine flow
- `third_party/servo/components/servo/soliloquy_bridge.rs` - typed snapshot bridge for the current `rv8` Servo/V8 work

## Architecture

At the top level, the repo currently breaks down like this:

- `src/shell/` - shell runtime, Servo embedder glue, V8 bridge, networking, platform code
- `src/` - browser optimization library: memory residency, cache, GPU compositor/layout, prefetching, V8 integration
- `src/rv8/` - experimental multi-process browser/runtime work with IPC, renderer, JS, parser, and optimization modules
- `sold/` - local HTTP service that serves the bundled UI and simple file/settings APIs
- `system/alpine/` - immutable-rootfs appliance assembly, service layout, artifact staging, and QEMU helpers
- `bundle/` - static UI assets served by `sold`
- `ui/desktop/` - Svelte frontend used by the dev flow and Alpine staging

## Current Bridge State

- Servo has a backend selection seam controlled by `SOLILOQUY_JS_ENGINE`
- `v8-experimental` is a real mode selection, but unsupported work still falls back to Servo's existing `mozjs` path
- the current bridge covers simple literals, `+` expressions, structured `window.__soliloquyEval(...)` calls, and live snapshot-backed reads/writes for a narrow DOM surface
- the live snapshot bridge has been extracted into `third_party/servo/components/servo/soliloquy_bridge.rs`
- `cargo test --manifest-path src/shell/Cargo.toml --lib` passes locally
- `bun run check` and `bun run build` pass locally in `ui/desktop`, with the same CSS compatibility warning as before
- Servo-side Rust validation is still blocked in this environment by the existing `mozangle` / Apple LLVM header issue

## Build Systems

Current build paths:

- `Cargo` - primary day-to-day Rust build/test path at the repo root
- `Bun` - JavaScript package manager and Svelte UI check/build path
- `system/alpine/scripts/*` - appliance staging and QEMU boot path

Treat Cargo plus Bun plus the Alpine scripts as the authoritative local path.

## Common Commands

Rust workspace:

```sh
cargo build
cargo test
cargo test -p soliloquy-shell
cargo test -p rv8
cargo test -p sold
```

Development flow:

```sh
./scripts/dev.sh
./scripts/dev.sh --shell-only
./scripts/dev.sh --ui-only
./scripts/dev.sh --qemu
```

Alpine/QEMU flow:

```sh
./system/alpine/scripts/setup-host.sh
./system/alpine/scripts/qemu-v0.sh
```

## Important Paths

- `Cargo.toml` - root workspace manifest
- `src/Cargo.toml` - optimization library crate
- `src/shell/Cargo.toml` - shell crate
- `src/rv8/Cargo.toml` - RV8 crate
- `sold/Cargo.toml` - local service crate
- `system/alpine/README.md` - most concrete appliance/runtime documentation
- `docs/` - broader architecture and contributor docs, some of which are stale

## Notes And Caveats

- The root workspace is not a 25-member workspace in this checkout.
- `third_party/servo` is a nested repository and should be treated as an actively patched fork for `rv8`.
- `system/alpine/scripts/sol-servo-wrapper` has had local work in this branch history; verify the current status before editing it.

## Working Assumptions

When changing code in this repo:

- Prefer existing Rust/Cargo patterns over reviving older platform assumptions from stale docs.
- Verify file paths before trusting documentation references.
- Keep Alpine appliance work, shell/runtime work, and side-project work scoped separately.
- Check `git status` before making changes because the workspace may already be dirty.
