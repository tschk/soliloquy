# Soliloquy Testing Documentation

Current test coverage is centered on Rust, Bun, the local `sold` bridge, and targeted Servo/RV8 checks.

## Quick Start

```bash
cargo test -p sold
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

The macOS smoke path compiles the non-GPUI desktop path, compiles the Crepuscularity GPUI desktop path, and dry-runs the desktop launch contract. It is intended for pre-launch validation and does not keep a GUI process open. Svelte and `sold` checks are appliance checks, not desktop browser checks.

For Servo/RV8 bridge checks:

```bash
./tools/rv8_servo_test.sh bridge
./tools/rv8_servo_test.sh v8
```

## Appliance Checks

```bash
sh -n system/alpine/scripts/*.sh
./system/alpine/scripts/qemu-v0.sh
```

Wayland appliance boot should show Cage/Wayland, `sold` health, and Servo startup in serial logs.
The default appliance config sets `SOL_SERVO_NO_BROWSER_CHROME=1`; visual checks should show only Soliloquy's Svelte appliance browser chrome, not Servo's built-in toolbar.
