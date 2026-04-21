# Soliloquy

Soliloquy is an experimental browser appliance built on Alpine Linux. It uses Servo as its rendering engine and V8 as its JavaScript runtime, targeting x86_64 and ARM64 Linux systems. The project is in early development and is not production-ready.

## Appliance Architecture

Soliloquy treats Alpine as a build substrate, not a distro:

- **Immutable rootfs**: SquashFS read-only root, tmpfs for writable data
- **Minimal system**: busybox, openrc, seatd, wlroots/cage, Servo, sold
- **No logins/shells**: Direct boot to browser session
- **Appliance model**: Single-purpose browser device

### Services
- **seatd**: Seat management
- **sold**: Local static file server (Rust/axum)
- **sol-session**: Launches cage with Servo fullscreen

### Filesystem
- `/` (squashfs): Immutable system
- `/tmp`, `/var/log` (tmpfs): Ephemeral
- `/var/lib/soliloquy`: Browser profile/state (persistent)

## Quick Start

### Development Mode
```bash
./scripts/dev.sh          # Start shell and UI dev server
./scripts/dev.sh --shell-only  # Shell only
./scripts/dev.sh --ui-only     # UI only
./scripts/dev.sh --qemu        # Build for QEMU
```

### Appliance Build
```bash
cd system/alpine
./setup-host.sh           # Setup build deps
./qemu-v0.sh             # Full build and run in QEMU
```

### Manual Build Steps
```bash
./build-rootfs.sh        # Build minimal Alpine rootfs
./ensure-servo-fork.sh   # Build Rust binaries (musl)
./configure-rootfs.sh    # Apply appliance configuration
./stage-soliloquy-artifacts.sh  # Stage binaries and bundle
```

## Build System

Alpine is used only for building the rootfs artifact:
- `apk` assembles minimal packages at build time
- Custom OpenRC services replace Alpine defaults
- Final image is squashfs + qcow2 for QEMU

At runtime, Alpine components are invisible - it's a pure appliance.

## Browser APIs

The Rust browser connects to Alpine kernel via standard Linux syscalls:
- **Networking**: reqwest/tokio (compatible with musl)
- **Display**: winit + WGPU (Wayland/X11)
- **Storage**: sled (embedded, no Alpine deps)
- **IPC**: ipc-channel (cross-platform)

Musl target ensures clean static linking and Alpine compatibility.

## Build Systems

- Cargo: cargo build / cargo test (workspace members)

## Project Layout

    src/shell/          Rust shell: servo_embedder, v8_runtime, engine_bridge, platform/*
    src/shell/net/      Networking: quic.rs, resource_loader.rs, connection_manager.rs,
                        speculation.rs, code_cache.rs
    src/rv8/            RV8 browser engine: core/, ipc/, renderer/, js/, servo_embed/
    src/memory/         Tab residency, compression, pressure monitoring, disk storage
    src/gpu/            layout_compute.rs, compositor.rs, wgpu_integration.rs
    src/cache/          unified.rs (LRU), texture_atlas.rs, disk_cache.rs
    docs/               ARCHITECTURE.md, browser_optimizations.md, build.md, guides/

## Key Crates and Dependencies

Shell (src/shell/Cargo.toml):
- rusty_v8 0.32 (V8 bindings), tokio 1.40 (async), serde/serde_json (serialization)
- Networking: quinn 0.11 (QUIC), rustls 0.23, hyper 1.4, reqwest 0.12

RV8 (src/rv8/):
- ipc-channel for real multi-process IPC (bootstrap server pattern)
- html5ever for HTML tokenization/parsing

Root workspace: 25 members, edition 2021, Apache 2.0 license

## Quick Start

### Development Mode
```bash
./scripts/dev.sh          # Start shell and UI dev server
./scripts/dev.sh --shell-only  # Shell only
./scripts/dev.sh --ui-only     # UI only
```

### QEMU Boot
```bash
./scripts/dev.sh --qemu   # Build and boot Rust system in QEMU
```

### Build
```bash
cargo build --release     # Build all Rust components
```

## Common Patterns

- License field: Use license-file.workspace = true (not license.workspace) in Cargo.toml files
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

- QUIC test_quic_connect is disabled (requires network access)
- GPU tests require WGPU-compatible hardware or will fall back to CPU

## Performance Targets

    Cold page load:       < 3.0s
    Warm page load:       < 1.0s
    Predicted navigation: < 0.2s
    Per-tab memory:       < 20MB
    150 tabs total:       < 3GB RAM
