# Soliloquy

Soliloquy is an experimental browser appliance built around a Rust shell, Servo integration work, a V8 runtime layer, and an Alpine-based runtime image. The repository also contains older architecture notes and adjacent experiments, but the code that is wired up today is primarily Linux/Alpine-focused.

This project is early-stage and not production-ready.

## What Is In This Repo

The root workspace currently contains four Rust packages:

- `src/` - `soliloquy_browser_optimizations`
- `src/shell/` - `soliloquy-shell`
- `src/rv8/` - `rv8`
- `sold/` - `sold`

Other important top-level areas:

- `system/alpine/` - appliance rootfs assembly, staging, and QEMU scripts
- `ui/desktop/` - Svelte desktop UI used by the dev flow and Alpine staging
- `bundle/` - static UI assets served by `sold`
- `soliloquy-web/` - Crepuscularity-based web UI/runtime experiments
- `third_party/servo/` - in-tree Servo checkout used by the Alpine flow
- `docs/` - project docs; some sections are current, some are historical

## Architecture Snapshot

- `soliloquy-shell` handles shell/runtime concerns, networking, platform integration, and the Servo/V8 bridge
- `soliloquy_browser_optimizations` provides cache, memory residency, GPU, network, and V8 support modules
- `rv8` is the experimental browser/runtime engine crate with IPC, rendering, parsing, and JS execution paths
- `sold` is a local Axum service that serves bundled UI assets and simple file/settings APIs
- `system/alpine` packages the runtime into an appliance-style Alpine image and boots it under QEMU

## Build And Run

### Rust workspace

```sh
cargo build
cargo test
```

Targeted packages:

```sh
cargo test -p soliloquy-shell
cargo test -p rv8
cargo test -p sold
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

More detail lives in [system/alpine/README.md](system/alpine/README.md).

## Current Build-System Reality

Several build systems appear in the repo, but they are not in the same state:

- `Cargo` is the clearest active path for local Rust work
- `system/alpine/scripts/*` is the clearest active path for appliance packaging and QEMU boot
- `Bazel` files exist, but they look partial
- older docs reference `GN/Ninja` and Zircon/Fuchsia-oriented layouts that are not represented by a root `BUILD.gn` in this checkout

## Audit Notes

During this pass, a few repo-level mismatches stood out:

- the previous root README described the project more narrowly than the codebase now warrants
- the previous `CLAUDE.md` claimed a 25-member workspace and a `backend/` tree that do not exist here
- `scripts/build.sh` still references a removed `backend/` target
- `docs/README.md` still links to missing translation/component-manifest material
- the working tree is already dirty in `system/alpine/scripts/sol-servo-wrapper` and `third_party/servo`

## Where To Look Next

- [CLAUDE.md](CLAUDE.md) for a concise repo-operating guide
- [system/alpine/README.md](system/alpine/README.md) for the appliance path
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [docs/architecture/architecture.md](docs/architecture/architecture.md) for broader design context
- [src/README.md](src/README.md) for the optimization library internals
