# AGENTS.md - Soliloquy Project Guide

## What is Soliloquy?

Soliloquy is the desktop environment and browser shell for the Alpenglow OS appliance. It owns the Rust shell, Servo integration work, V8/RV8 runtime bridge, browser chrome, UI bundle, desktop controls, and browser-facing optimization code.

The installable operating system, rootfs composition, kernel policy, GlowFS module, service graph, board support, and `sold` system bridge live in `../alpenglow`.

## Architecture

Soliloquy Desktop ------ Browser shell and desktop controls
  Servo Surface -------- Fullscreen browser shell loading the desktop bundle
  RV8 Runtime ---------- Servo/V8 bridge work and RV8 browser engine path
  Memory Manager ------- Tiered tab residency and memory-pressure behavior
  GPU Rendering -------- WGPU compute shaders and damage-based compositing
  Cache System --------- LRU, disk cache, V8 bytecode, texture atlas
RV8 Browser Engine ----- Multi-process browser engine in `../rv8`
Alpenglow OS ----------- Installable OS, kernel policy, rootfs, services in `../alpenglow`

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
    src/memory/         Tab residency, compression, pressure monitoring, disk storage
    src/gpu/            layout_compute.rs, compositor.rs, wgpu_integration.rs
    src/cache/          unified.rs (LRU), texture_atlas.rs, disk_cache.rs
    ui/desktop/         Svelte desktop environment and browser chrome
    third_party/servo/  Servo checkout and local bridge work
    tools/              RV8/Servo checks, UI build/dev/start helpers
    docs/               Architecture, build, browser, and development docs
    ../rv8/             Canonical RV8 browser engine
    ../alpenglow/       Installable OS layer

## Testing

    cargo test
    cargo test -p soliloquy-shell --lib
    cargo test -p soliloquy_browser_optimizations
    ./tools/rv8_servo_test.sh bridge
    cd ui/desktop && bun run check && bun run build

## Boundaries

- Do not add OS image, kernel, service-manager, rootfs, board, or installer work back into this repo.
- Put installable OS changes in `../alpenglow`.
- Keep `../rv8` as the browser-engine boundary.
