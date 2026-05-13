# Developer Guide

## Local Loop

```bash
cargo test -p sold
./tools/soliloquy/start.sh
```

Open `http://127.0.0.1:8080`.

## UI Loop

```bash
cd ui/desktop
bun run check
bun run build
```

## macOS Desktop Loop

```bash
./tools/soliloquy/smoke_macos.sh
```

This is the non-persistent macOS load check. It checks the non-GPUI desktop binary and dry-runs the browser-only Servo launch contract. It does not start `sold` or use the Svelte appliance chrome.

## Runtime Loop

```bash
./tools/rv8_servo_test.sh bridge
./tools/rv8_servo_test.sh v8
```

## Appliance Loop

```bash
./system/alpine/scripts/setup-host.sh
./system/alpine/scripts/qemu-v0.sh
```
