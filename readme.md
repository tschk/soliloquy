# Soliloquy

Soliloquy is an experimental browser appliance built around a Rust shell, Servo integration work, a V8 runtime layer, and an Alpine-based runtime image. The repository also contains older architecture notes and adjacent experiments, but the code that is wired up today is primarily Linux/Alpine-focused.

This project is early-stage and not production-ready.

## What Is In This Repo

The root workspace currently contains these Rust packages:

- `src/` - `soliloquy_browser_optimizations`
- `src/shell/` - `soliloquy-shell`
- `sold/` - `sold`
- `drivers/generic/` - `soliloquy-drivers`
- `system/kernelctl/` - `sol-kernelctl`
- `system/netd/` - `sol-netd`
- `system/solfsctl/` - `solfsctl`

Other important top-level areas:

- `system/alpine/` - appliance rootfs assembly, staging, and QEMU scripts
- `ui/desktop/` - Svelte appliance UI used by the dev flow and Alpine staging
- `bundle/` - supplemental static assets served by `sold`, including the terminal page and Ghostty VT bundle
- `third_party/servo/` - in-tree Servo checkout used by the Alpine flow
- `docs/` - project docs for the current Cargo, Bun, Alpine, Servo/RV8, and `sold` paths

## Architecture Snapshot

- `soliloquy-shell` handles shell/runtime concerns, networking, platform integration, and the Servo/V8 bridge
- `soliloquy_browser_optimizations` provides cache, memory residency, GPU, network, and V8 support modules
- `../rv8` is the sibling experimental browser/runtime engine checkout with IPC, rendering, parsing, and JS execution paths
- `sold` is a local Axum service that serves bundled UI assets and simple file/settings APIs
- `system/alpine` packages the runtime into an appliance-style Alpine image and boots it under QEMU
- `third_party/servo` is an in-tree Servo checkout with local `rv8` bridge patches and a typed snapshot bridge module

## Build And Run

### Rust workspace

```sh
cargo build
cargo test
```

Targeted packages:

```sh
cargo test -p soliloquy-shell
cargo test -p sold
cargo test -p sol-netd
cargo test -p sol-kernelctl
```

### Local dev flow

```sh
./scripts/dev.sh
./scripts/dev.sh --shell-only
./scripts/dev.sh --ui-only
./scripts/dev.sh --qemu
```

`./scripts/dev.sh` starts the Rust shell and the Svelte UI dev server from `ui/desktop/`.

### Alpine appliance / QEMU flow

```sh
./system/alpine/scripts/setup-host.sh
./system/alpine/scripts/qemu-v0.sh
```

`qemu-v0.sh` is the canonical appliance entry point. It builds the Svelte bundle with Bun, prepares Linux runtime binaries for the selected `QEMU_ARCH`, stages the current `ui/desktop/build` output at `/usr/local/share/soliloquy/bundle`, adds the supplemental `bundle/terminal` assets, builds the rootfs image, and launches QEMU unless `QEMU_RUN=0` is set.

More detail lives in [system/alpine/README.md](system/alpine/README.md).

## Current Build-System Reality

Current build paths:

- `Cargo` is the clearest active path for local Rust work
- `system/alpine/scripts/*` is the clearest active path for appliance packaging and QEMU boot
- `Bun` is the only JavaScript package manager used for the Svelte UI

## Current Bridge State

- Servo has a backend selection seam controlled by `SOLILOQUY_JS_ENGINE`
- `v8-experimental` is a real mode, but unsupported work still falls back to Servo's existing `mozjs` path
- the current bridge covers simple literals, `+` expressions, structured `window.__soliloquyEval(...)` calls, and live snapshot-backed reads/writes for a narrow DOM surface
- the live snapshot bridge has been extracted into `third_party/servo/components/servo/soliloquy_bridge.rs`
- `cargo test --manifest-path src/shell/Cargo.toml --lib` passes locally
- `bun run check` and `bun run build` pass locally in `ui/desktop`, with the same CSS compatibility warning as before
- Servo-side Rust validation is still blocked in this environment by the existing `mozangle` / Apple LLVM header issue

## Where To Look Next

- [CLAUDE.md](CLAUDE.md) for a concise repo-operating guide
- [system/alpine/README.md](system/alpine/README.md) for the appliance path
- [docs/v0-architecture.md](docs/v0-architecture.md) and [docs/architecture/appliance-system.md](docs/architecture/appliance-system.md) for broader design context
- [src/README.md](src/README.md) for the optimization library internals
- [docs/rv8_linkage_roadmap.md](docs/rv8_linkage_roadmap.md) for the current bridge plan and remaining work
