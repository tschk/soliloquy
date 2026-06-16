# Soliloquy

Soliloquy is the desktop environment and browser shell for the Alpenglow OS appliance. It owns the Rust shell, Servo integration work, V8/RV8 runtime bridge, browser chrome, UI bundle, desktop controls, and browser-facing optimization code.

The installable operating system, rootfs composition, kernel policy, service graph, board support, and netd live in `../alpenglow-os`.

This project is early-stage and not production-ready.

## What Is In This Repo

The root workspace currently contains these Rust packages:

- `src/` - `soliloquy_browser_optimizations`
- `src/shell/` - `soliloquy-shell` (desktop shell + browser)
- `src/desktop/` - `soliloquy-daemon` (DE daemon, consumes alpenglow OS state)

Other important top-level areas:

- `ui/desktop/` - Svelte desktop environment and browser chrome
- `third_party/servo/` - in-tree Servo checkout used by the desktop shell flow
- `docs/` - project docs for the current Cargo, Bun, Servo/RV8, and desktop paths
- `tools/soliloquy/` - desktop build/dev/start helpers

## Architecture Snapshot

- `soliloquy-shell` handles shell/runtime concerns, platform integration, and the Servo/V8 bridge
- `soliloquy_browser_optimizations` provides cache, memory residency, GPU, network, and V8 support modules
- `soliloquy-daemon` is the desktop environment daemon — reads alpenglow OS state (network, kernel), manages apps/sessions, exposes an HTTP API for the Svelte UI
- `../rv8` is the sibling experimental browser/runtime engine checkout with IPC, rendering, parsing, and JS execution paths
- `ui/desktop` provides the appliance desktop surface that Alpenglow stages into its image
- `../alpenglow-os` owns OS packaging, install, kernel policy, rootfs generation, target-board boot, and the system daemons (`alpenglow-netd`, `oil`)

**Boundary**: Soliloquy consumes `alpenglow-netd` for state types (from `../alpenglow-os`) but owns all desktop-environment concerns — windowing, browser shell, app launcher, session management, Svelte UI.

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
cargo test -p soliloquy-daemon
```

### QEMU boot (fast dev loop)

Build the Alpine-based Soliloquy image and boot it in QEMU with one command:

```sh
./start.sh                         # build + boot QEMU (headless)
./start.sh --headless              # serial console mode
./start.sh --build-only            # prepare image, skip VM
```

On Apple Silicon, the boot uses `qemu-system-aarch64` with HVF acceleration 
and reaches the `soliloquy-daemon` in **&lt;1 second**:

```
[    0.000000] Booting Linux on physical CPU
[    0.267349] Run /init as init process
sh: can't access tty; job control turned off
2026-06-16T05:37:24Z INFO soliloquy_daemon: listening on 127.0.0.1:9842
```

The image is built from `../alpenglow-os` (Alpine Linux base + kernel) with 
the `soliloquy-daemon` binary cross-compiled for aarch64-musl and a minimal 
init that skips OpenRC — just mounts filesystems and starts the daemon directly.

### Desktop bundle

```sh
cd ui/desktop && bun run build
```

### Current build paths

- `Cargo` — local Rust desktop work
- `Bun` — Svelte UI bundle
- `../alpenglow-os` — OS install, QEMU, kernel, image gates

## Current Bridge State

- Servo has a backend selection seam controlled by `SOLILOQUY_JS_ENGINE`
- `v8-experimental` is a real mode, but unsupported work still falls back to Servo's existing `mozjs` path
- the current bridge covers simple literals, `+` expressions, structured `window.__soliloquyEval(...)` calls, and live snapshot-backed reads/writes for a narrow DOM surface
- the live snapshot bridge has been extracted into `third_party/servo/components/servo/soliloquy_bridge.rs`
- `cargo test --manifest-path src/shell/Cargo.toml --lib` passes locally
