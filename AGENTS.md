# AGENTS.md - Soliloquy Project Guide

## What is Soliloquy?

Soliloquy is an experimental, web-native operating system built on the Zircon microkernel (from Fuchsia). It uses Servo as its rendering engine and V8 as its JavaScript runtime, targeting the Radxa Cubie A5E (Allwinner A527 ARM64 SBC). The project is in early development and is not production-ready.

## Architecture

Soliloquy Shell (Rust) --- Desktop shell, window management
  Servo Embedder ---------- HTML fetching, parsing, rendering via Servo + html5ever
  V8 Runtime -------------- JavaScript execution via rusty_v8
  Networking -------------- HTTP/3 QUIC (quinn), HTTP/1.1+2 (reqwest/hyper)
  Memory Manager ---------- Tiered tab residency (<3GB for 150+ tabs)
  GPU Rendering ----------- WGPU compute shaders, damage-based compositing
  Cache System ------------ LRU + disk + V8 bytecode + texture atlas
  Platform Layer ---------- Fuchsia (Flatland), Linux (winit+WGPU), macOS
RV8 Browser Engine (Rust) - Multi-process browser with IPC via ipc-channel
Backend (V language) ------ Cupboard memory storage, vweb server
Drivers (Rust) ------------ AIC8800 WiFi, GPIO, Mali G57 GPU (stubs)
Zircon translations (V) --- C-to-V translated kernel subsystems

## Build Systems

Three build systems coexist:

- GN/Ninja (primary for Fuchsia targets): gn gen out/default && ninja -C out/default
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
    backend/            V language: cupboard.v (memory storage with inverted index), main.v
    drivers/            wifi/aic8800, gpio, gpu/mali_g57, common/soliloquy_hal
    boards/             arm64/soliloquy (A527 board support)
    third_party/        fuchsia-sdk-rust/, zircon_v/ (C-to-V translated: vm, ipc, scenic)
    gen/fidl/           Generated FIDL bindings (fuchsia_ui_composition, views, app, input)
    tools/              build_manager (Rust CLI), scripts, soliloquy flash/boot utils
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
- Platform conditional: V code uses  fuchsia ? for Fuchsia-specific paths
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
- V language backend cannot be compiled without the V toolchain (built locally for ARM64)
- GPU tests require WGPU-compatible hardware or will fall back to CPU

## Performance Targets

    Cold page load:       < 3.0s
    Warm page load:       < 1.0s
    Predicted navigation: < 0.2s
    Per-tab memory:       < 20MB
    150 tabs total:       < 3GB RAM

## Backend: Cupboard Memory System

The V language backend (backend/cupboard.v) implements a memory storage system with:
- Inverted index for ~20x faster search (tokenize -> intersect candidate IDs)
- Background write worker using channel-based I/O (file kept open, append mode)
- User memory index for O(1) user-scoped lookups
- REST endpoints: /api/cupboard/store, /api/cupboard/retrieve, /api/cupboard/delete, /api/cupboard/stats
