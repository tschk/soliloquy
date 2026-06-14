# Soliloquy Testing Documentation

Current test coverage is centered on Rust, Bun, the desktop shell, and targeted Servo/RV8 checks.

## Quick Start

```bash
cargo test -p soliloquy-shell --lib
```

The sibling RV8 repo is available at `../rv8`:

```bash
cargo test --manifest-path ../rv8/Cargo.toml
```

For UI checks:

```bash
cd ui/desktop
bun run check
bun run build
```

For macOS desktop shell smoke checks:

```bash
./tools/soliloquy/smoke_macos.sh
```

The macOS smoke path compiles the non-GPUI desktop path, compiles the Crepuscularity GPUI desktop path, and dry-runs the desktop launch contract. It is intended for pre-launch validation and does not keep a GUI process open.
Because the smoke command uses `SOL_MACOS_DRY_RUN=1`, it does not start `sold`. A real `tools/soliloquy/start_macos.sh` launch can start or reuse `sold` for runtime APIs while still avoiding the Svelte appliance chrome.

For Servo/RV8 bridge checks:

```bash
./tools/rv8_servo_test.sh bridge
./tools/rv8_servo_test.sh v8
```

## Appliance Checks

```bash
cd ../alpenglow
./install.sh --check
```

Alpenglow consumes Soliloquy's `ui/desktop/build` output when composing an OS image.
