# Build

Soliloquy uses Cargo for Rust and Bun for the Svelte UI. Alpenglow owns appliance staging and install in `../alpenglow`.

## Rust

```bash
cargo build
cargo test -p soliloquy-shell --lib
```

The sibling RV8 repo is checked directly:

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

## macOS Desktop Smoke

```bash
./tools/soliloquy/smoke_macos.sh
```

This checks the non-GPUI desktop binary, checks the Crepuscularity GPUI desktop binary, and dry-runs the macOS launch contract. The supported macOS desktop path is Crepuscularity chrome plus Servo with `--no-browser-chrome`; it must not load the Svelte appliance chrome. The Svelte desktop bundle belongs to the Alpine appliance path.
The smoke script sets `SOL_MACOS_DRY_RUN=1`, so it checks the launch contract without starting `sold` or opening a persistent GUI.

To launch the macOS browser:

```bash
./tools/soliloquy/start_macos.sh
```

The real macOS launcher starts or reuses `sold` for local runtime APIs, then launches the Crepuscularity GPUI chrome with Servo's built-in browser chrome disabled. It still does not serve the Svelte appliance chrome.

## Local Runtime

```bash
./tools/soliloquy/start.sh
```

The appliance session defaults to `SOL_SERVO_NO_BROWSER_CHROME=1`. Keep Soliloquy's Svelte appliance shell as the single visible browser chrome; Servo should render page content only. Browser UI modes are available from the appliance chrome: Zen sidebar, compact, split columns, split rows, and grid.

## Alpenglow OS

```bash
cd ../alpenglow
./install.sh --check
```

Build `ui/desktop/build` here before composing an Alpenglow image. Alpenglow stages that bundle into the OS image.
