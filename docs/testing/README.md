# Soliloquy Testing Documentation

Current test coverage is centered on Rust, Bun, the local `sold` bridge, and targeted Servo/RV8 checks.

## Quick Start

```bash
cargo test -p sold
cargo test -p soliloquy-shell --lib
cargo test -p rv8
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

The macOS smoke path checks the UI and compiles `soliloquy_desktop` with `--features gpui` against `../crepuscularity/crates/crepuscularity-gpui`. It is intended for pre-launch validation and does not keep a GUI process open.

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
The default appliance config sets `SOL_SERVO_NO_BROWSER_CHROME=1`; visual checks should show only Soliloquy's desktop browser chrome, not Servo's built-in toolbar.
