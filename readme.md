# Soliloquy

Soliloquy is the desktop environment and browser shell for the Alpenglow OS appliance. It owns the Rust shell, Servo integration work, V8/RV8 runtime bridge, browser chrome, UI bundle, desktop controls, and browser-facing optimization code.

The installable operating system, rootfs composition, kernel policy, GlowFS module, service graph, board support, and `sold` system bridge now live in `../alpenglow` and [tschk/alpenglow](https://github.com/tschk/alpenglow).

This project is early-stage and not production-ready.

## What Is In This Repo

The root workspace currently contains these Rust packages:

- `src/` - `soliloquy_browser_optimizations`
- `src/shell/` - `soliloquy-shell`

Other important top-level areas:

- `ui/desktop/` - Svelte desktop environment and browser chrome
- `third_party/servo/` - in-tree Servo checkout used by the desktop shell flow
- `docs/` - project docs for the current Cargo, Bun, Servo/RV8, and desktop paths
- `tools/soliloquy/` - desktop build/dev/start helpers

## Architecture Snapshot

- `soliloquy-shell` handles shell/runtime concerns, platform integration, and the Servo/V8 bridge
- `soliloquy_browser_optimizations` provides cache, memory residency, GPU, network, and V8 support modules
- `../rv8` is the sibling experimental browser/runtime engine checkout with IPC, rendering, parsing, and JS execution paths
- `ui/desktop` provides the appliance desktop surface that Alpenglow stages into its image
- `../alpenglow` owns OS packaging, install, kernel-level modifications, rootfs generation, `sold`, and target-board boot

## Build And Run

### Rust workspace

```sh
cargo build
cargo test
```

Targeted packages:

```sh
cargo test -p soliloquy-shell --lib
cargo test -p soliloquy_browser_optimizations
```

### Local dev flow

```sh
./scripts/dev.sh
./scripts/dev.sh --shell-only
./scripts/dev.sh --ui-only
```

`./scripts/dev.sh` starts the Rust shell and the Svelte UI dev server from `ui/desktop/`.

### Desktop bundle

```sh
./tools/soliloquy/build_ui.sh
```

Alpenglow consumes the generated `ui/desktop/build` bundle when composing an OS image.

## Current Build-System Reality

Current build paths:

- `Cargo` is the active path for local Rust desktop work
- `Bun` is the only JavaScript package manager used for the Svelte UI
- `../alpenglow` owns OS install, QEMU, kernel, and image gates

## Current Bridge State

- Servo has a backend selection seam controlled by `SOLILOQUY_JS_ENGINE`
- `v8-experimental` is a real mode, but unsupported work still falls back to Servo's existing `mozjs` path
- the current bridge covers simple literals, `+` expressions, structured `window.__soliloquyEval(...)` calls, and live snapshot-backed reads/writes for a narrow DOM surface
- the live snapshot bridge has been extracted into `third_party/servo/components/servo/soliloquy_bridge.rs`
- `cargo test --manifest-path src/shell/Cargo.toml --lib` passes locally
