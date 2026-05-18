# AGENTS.md - Soliloquy Project Guide

## What is Soliloquy?

Soliloquy is an experimental, web-native operating system appliance built on Alpine Linux. It uses Servo as its fullscreen rendering engine, RV8/V8 for the JavaScript runtime path, and `sold` as the local system bridge, targeting the Radxa Cubie A5E (Allwinner A527 ARM64 SBC). The project is in early development and is not production-ready.

## Architecture

Alpine Appliance -------- Immutable base image, OpenRC service startup
  Servo Surface ---------- Fullscreen browser shell loading the desktop bundle
  sold Bridge ------------ Local authenticated system and terminal APIs
  RV8 Runtime ------------ Servo/V8 bridge work and RV8 browser engine path
  Networking ------------- Linux networking plus local resource loading
  Memory Manager --------- Tiered tab residency (<3GB for 150+ tabs)
  GPU Rendering ---------- WGPU compute shaders, damage-based compositing
  Cache System ----------- LRU + disk + V8 bytecode + texture atlas
RV8 Browser Engine (Rust) - Multi-process browser with IPC via ipc-channel
Drivers (Rust) ----------- AIC8800 WiFi, GPIO, Mali G57 GPU (stubs)

## Build Systems

Build paths currently coexist:

- Alpine image assembly and OpenRC service staging under `system/alpine`
- Servo/RV8 bridge checks through Cargo and the local Servo checkout
- Cargo: cargo build / cargo test
- Bun: bun run check / bun run build in `ui/desktop`

## Project Layout

    src/shell/          Rust shell: servo_embedder, v8_runtime, engine_bridge, platform/*
    src/shell/net/      Networking: quic.rs, resource_loader.rs, connection_manager.rs,
                        speculation.rs, code_cache.rs
    src/rv8/            RV8 browser engine: core/, ipc/, renderer/, js/, servo_embed/
    src/memory/         Tab residency, compression, pressure monitoring, disk storage
    src/gpu/            layout_compute.rs, compositor.rs, wgpu_integration.rs
    src/cache/          unified.rs (LRU), texture_atlas.rs, disk_cache.rs
    drivers/            wifi/aic8800, gpio, gpu/mali_g57, common/soliloquy_hal
    boards/             arm64/soliloquy (A527 board support)
    third_party/        servo/ checkout and optional external source trees
    gen/fidl/           Generated FIDL bindings (fuchsia_ui_composition, views, app, input)
    tools/              RV8/Servo checks, UI build/dev/start helpers, serial debug
    test/               support lib, component tests, vm tests
    docs/               v0-architecture.md, browser_optimizations.md, build.md, guides/

## Key Crates and Dependencies

Shell (src/shell/Cargo.toml):
- rusty_v8 0.32 (V8 bindings), tokio 1.40 (async), serde/serde_json (serialization)
- Networking: quinn 0.11 (QUIC), rustls 0.23, hyper 1.4, reqwest 0.12
- FIDL stubs in gen/fidl/ (mock impls for host builds)

RV8 (src/rv8/):
- ipc-channel for real multi-process IPC (bootstrap server pattern)
- html5ever for HTML tokenization/parsing

Root workspace: 3 listed members, edition 2021, MPL-2.0 license

## Common Patterns

- License field: Use license-file.workspace = true (not license.workspace) in Cargo.toml files
- FIDL stubs: The gen/fidl/ crates are mock implementations
  for building on host systems. They need trait derives (Clone, Debug, PartialEq) and constants
  (PROTOCOL_NAME) added as needed.
- Borrow checker in speculation.rs: evaluate_speculation collects actions into a Vec before
  calling mutable methods to avoid borrow conflicts
- Async bridging: pollster::block_on wraps async WGPU calls in sync contexts;
  std::thread::spawn bridges blocking IPC receivers to tokio mpsc channels

## Testing

    cargo test                        # All Rust tests
    cargo test -p soliloquy-shell     # Shell crate only
    cargo test -p soliloquy-shell --lib  # Library tests only
Networking module has 56+ tests. Memory, GPU, and cache modules have their own test suites.
The shell test_flatland_present validates Flatland compositor integration.

## Known Issues

- fuchsia-component has a trait bound error that blocks some integration tests (pre-existing)
- QUIC test_quic_connect is disabled (requires network access)
- GPU tests require WGPU-compatible hardware or will fall back to CPU

## Performance Targets

    Cold page load:       < 3.0s
    Warm page load:       < 1.0s
    Predicted navigation: < 0.2s
    Per-tab memory:       < 20MB
    150 tabs total:       < 3GB RAM


<claude-mem-context>
# Memory Context

# claude-mem status

This project has no memory yet. The current session will seed it; subsequent sessions will receive auto-injected context for relevant past work.

Memory injection starts on your second session in a project.

`/learn-codebase` is available if the user wants to front-load the entire repo into memory in a single pass (~5 minutes on a typical repo, optional). Otherwise memory builds passively as work happens.

Live activity: http://localhost:37701
How it works: `/how-it-works`

This message disappears once the first observation lands.
</claude-mem-context>
