# AGENTS.md - Soliloquy Project Guide

## What is Soliloquy?

Soliloquy is the desktop environment and browser shell for the Alpenglow OS appliance. It owns the Rust shell, Servo integration work, V8/RV8 runtime bridge, browser chrome, UI bundle, desktop controls, and browser-facing optimization code.

The installable operating system, rootfs composition, kernel policy, service graph, board support, and system daemons live in `../alpenglow`.

## Architecture

Soliloquy Desktop ------ Browser shell and desktop controls
  DE Daemon (sold) ----- Background daemon consuming alpenglow OS state (netd, kernelctl),
                         managing apps/sessions, exposing HTTP API for Svelte UI
  Servo Surface -------- Fullscreen browser shell loading the desktop bundle
  RV8 Runtime ---------- Servo/V8 bridge work and RV8 browser engine path
  Memory Manager ------- Tiered tab residency and memory-pressure behavior
  GPU Rendering -------- WGPU compute shaders and damage-based compositing
  Cache System --------- LRU, disk cache, V8 bytecode, texture atlas
RV8 Browser Engine ----- Multi-process browser engine in `../rv8`
Alpenglow OS ----------- OS daemons (netd-zig, kernelctl-zig, oil), kernel policy,
                         rootfs, initramfs, service graph in `../alpenglow`

Consumption boundary: Soliloquy reads daemon output files (`/run/alpenglow/...`) at runtime.
  No Rust crate dependency on alpenglow — state files parsed locally.
  Do not add OS daemon logic to soliloquy.

## Build Systems

Build paths currently coexist:

- Cargo: `cargo build` / `cargo test`
- Bun: `bun run check` / `bun run build` in `ui/desktop`
- Servo/RV8 bridge checks through Cargo and the local Servo checkout
- OS image and install checks through `../alpenglow`

## Project Layout

    src/shell/          Rust shell: servo_embedder, v8_runtime, engine_bridge, platform/*
    src/shell/net/      Networking: quic.rs, resource_loader.rs, connection_manager.rs,
                        speculation.rs, code_cache.rs
    src/desktop/        DE daemon (soliloquy-daemon / sold): consumes alpenglow netd/kernelctl state,
                        app registry, session manager, Axum HTTP API for Svelte UI
    src/                Browser optimization stubs (rv8 integration handled by sibling agent)
    ui/desktop/         Svelte desktop environment and browser chrome
    third_party/servo/  Servo checkout and local bridge work
    tools/              RV8/Servo checks, UI build/dev/start helpers
    docs/               Architecture, build, browser, and development docs
    build/              Custom alpenglow build overlay (replaces alpenglowed with sold)
    ../rv8/             Canonical RV8 browser engine
    ../alpenglow/       Alpenglow OS (netd-zig, kernelctl-zig, oil, kernel policy, rootfs, initramfs, boot)

## Testing

    cargo test
    cargo test -p soliloquy-shell --lib
    cargo test -p soliloquy-daemon
    cargo test -p soliloquy_browser_optimizations
    # Build DE daemon only
    cargo build -p soliloquy-daemon
    # Run DE daemon (background, reads /run/alpenglow/* if available)
    SOLILOQUY_DAEMON_PORT=9842 cargo run -p soliloquy-daemon -- --once
    ./tools/rv8_servo_test.sh bridge
    cd ui/desktop && bun run check && bun run build

## Boundaries

- Do not add OS image, kernel, service-manager, rootfs, board, or installer work back into this repo.
- Put installable OS changes in `../alpenglow`.
- Keep `../rv8` as the browser-engine boundary.
- Soliloquy reads alpenglow state files at runtime (`/run/alpenglow/...`).
  OS daemon logic stays in `../alpenglow/system/{netd-zig,kernelctl-zig,oil}`.
