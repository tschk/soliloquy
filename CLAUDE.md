# CLAUDE.md - Soliloquy Project Guide

## What Is Soliloquy?

Soliloquy is an experimental browser appliance and web-native shell. The current repo is centered on a Linux/Alpine runtime, a Rust shell, Servo integration work, a V8 runtime layer, and local UI/services for a single-purpose browser environment. The codebase also contains older architectural material that references Zircon/Fuchsia-style concepts, but the checked-in build and runtime paths in this repository are primarily Linux-oriented.

The project is early-stage and not production-ready.

## Current Repo Shape

There are four active Rust packages in the root Cargo workspace:

- `soliloquy_browser_optimizations` (`src/`) - shared cache, memory, GPU, network, and V8 support code
- `soliloquy-shell` (`src/shell/`) - shell library plus `soliloquy_shell` and `soliloquy_shell_simple` binaries
- `rv8` (`src/rv8/`) - experimental browser engine/runtime crate
- `sold` (`sold/`) - local Axum service for bundled UI, files, and settings

There are also substantial adjacent projects in-tree, including:

- `system/alpine/` - Alpine appliance build, staging, and QEMU boot flow
- `ui/desktop/` - Svelte desktop UI
- `soliloquy-web/` - Crepuscularity-based web UI/runtime experiments
- `crepuscularity/` and `equilibrium/` - separate subprojects with their own docs and manifests
- `third_party/servo/` - in-tree Servo checkout used by the Alpine flow

## Architecture

At the top level, the repo currently breaks down like this:

- `src/shell/` - shell runtime, Servo embedder glue, V8 bridge, networking, platform code
- `src/` - browser optimization library: memory residency, cache, GPU compositor/layout, prefetching, V8 integration
- `src/rv8/` - experimental multi-process browser/runtime work with IPC, renderer, JS, parser, and optimization modules
- `sold/` - local HTTP service that serves the bundled UI and simple file/settings APIs
- `system/alpine/` - immutable-rootfs appliance assembly, service layout, artifact staging, and QEMU helpers
- `bundle/` - static UI assets served by `sold`
- `ui/desktop/` - Svelte frontend used by the dev flow and Alpine staging

## Build Systems

Multiple build systems exist, but they are not equally current:

- `Cargo` - primary day-to-day Rust build/test path at the repo root
- `Bazel` - present via `MODULE.bazel` and `WORKSPACE.bazel`, but appears partial/legacy
- `GN/Ninja` - referenced in older docs, but there is no root `BUILD.gn` in this checkout

Treat Cargo plus the Alpine scripts as the authoritative local path unless you have a specific reason to work on Bazel or older architecture experiments.

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
- The repo does not currently contain the `backend/` tree described in some older docs.
- `scripts/build.sh` still references a removed `backend/` target and should be treated carefully.
- `docs/README.md` still points at missing translation/component-manifest material.
- `third_party/servo` is a nested repository and currently has local modifications; do not assume it is clean.
- `system/alpine/scripts/sol-servo-wrapper` is also modified in the working tree.

## Working Assumptions

When changing code in this repo:

- Prefer existing Rust/Cargo patterns over reviving older V/Fuchsia-era assumptions from stale docs.
- Verify file paths before trusting documentation references.
- Keep Alpine appliance work, shell/runtime work, and side-project work scoped separately.
- Check `git status` before making changes because the workspace may already be dirty.
