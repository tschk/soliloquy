# Soliloquy Testing Documentation

Current test coverage is centered on Rust, Bun, the local `sold` bridge, and targeted Servo/RV8 checks.

## Quick Start

```bash
cargo test -p sold
cargo test -p soliloquy-shell --lib
cargo test -p rv8
```

For UI checks:

```bash
cd ui/desktop
bun run check
bun run build
```

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
