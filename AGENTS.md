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
- Bazel: bazel build //... or bazel test //... (uses rules_rust v0.56.0)
- Cargo: cargo build / cargo test (25 workspace members)

The root BUILD.gn defines targets: :default, :soliloquy, :soliloquy_headless, :soliloquy_qemu, :tests, :drivers.

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
    third_party/        servo/, fuchsia-sdk-rust/ mock bindings
    gen/fidl/           Generated FIDL bindings (fuchsia_ui_composition, views, app, input)
    tools/              RV8/Servo checks, UI build/dev/start helpers, serial debug
    test/               support lib, component tests, vm tests
    docs/               ARCHITECTURE.md, browser_optimizations.md, build.md, guides/

## Key Crates and Dependencies

Shell (src/shell/Cargo.toml):
- rusty_v8 0.32 (V8 bindings), tokio 1.40 (async), serde/serde_json (serialization)
- Networking: quinn 0.11 (QUIC), rustls 0.23, hyper 1.4, reqwest 0.12
- FIDL stubs in third_party/fuchsia-sdk-rust/ and gen/fidl/ (mock impls for non-Fuchsia builds)

RV8 (src/rv8/):
- ipc-channel for real multi-process IPC (bootstrap server pattern)
- html5ever for HTML tokenization/parsing

Root workspace: 25 members, edition 2021, Apache 2.0 license

## Common Patterns

- License field: Use license-file.workspace = true (not license.workspace) in Cargo.toml files
- FIDL stubs: The gen/fidl/ and third_party/fuchsia-sdk-rust/ crates are mock implementations
  for building on non-Fuchsia. They need trait derives (Clone, Debug, PartialEq) and constants
  (PROTOCOL_NAME) added as needed.
- Borrow checker in speculation.rs: evaluate_speculation collects actions into a Vec before
  calling mutable methods to avoid borrow conflicts
- Async bridging: pollster::block_on wraps async WGPU calls in sync contexts;
  std::thread::spawn bridges blocking IPC receivers to tokio mpsc channels

## Testing

    cargo test                        # All Rust tests
    cargo test -p soliloquy-shell     # Shell crate only
    cargo test -p soliloquy-shell --lib  # Library tests only
    bazel test //...                  # All Bazel tests

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

# [soliloquy] recent context, 2026-05-05 11:57pm GMT+10

No previous sessions found.
</claude-mem-context>
