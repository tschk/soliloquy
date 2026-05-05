# Build

Soliloquy uses Cargo for Rust, Bun for the Svelte UI, and `system/alpine/scripts` for appliance staging.

## Rust

```bash
cargo build
cargo test -p sold
cargo test -p soliloquy-shell --lib
cargo test -p rv8
```

## UI

```bash
cd ui/desktop
bun install
bun run check
bun run build
```

## Local Runtime

```bash
./tools/soliloquy/start.sh
```

## Alpine Appliance

```bash
./system/alpine/scripts/setup-host.sh
./system/alpine/scripts/qemu-v0.sh
```
