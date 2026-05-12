# Build

Soliloquy uses Cargo for Rust, Bun for the Svelte UI, and `system/alpine/scripts` for appliance staging.

## Rust

```bash
cargo build
cargo test -p sold
cargo test -p soliloquy-shell --lib
cargo test -p rv8
```

The standalone sibling RV8 repo can also be checked directly:

```bash
cargo test --manifest-path ../rv8/Cargo.toml
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

The appliance session defaults to `SOL_SERVO_NO_BROWSER_CHROME=1`. Keep Soliloquy's desktop shell as the single visible browser chrome; Servo should render page content only. Browser UI modes are available from the desktop chrome: Zen sidebar, compact, split columns, split rows, and grid.

## Alpine Appliance

```bash
./system/alpine/scripts/setup-host.sh
./system/alpine/scripts/qemu-v0.sh
```
