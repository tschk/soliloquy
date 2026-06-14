# Developer Guide

## Local Loop

```bash
cargo test -p soliloquy-shell --lib
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

This is the non-persistent macOS load check. It checks the non-GPUI desktop binary, checks the Crepuscularity GPUI desktop binary, and dry-runs the desktop launch contract. It does not start `sold` or use the Svelte appliance chrome.
The real `tools/soliloquy/start_macos.sh` launcher can start or reuse `sold` for runtime APIs; the smoke path intentionally stops before that.

## Runtime Loop

```bash
./tools/rv8_servo_test.sh bridge
./tools/rv8_servo_test.sh v8
```

## Appliance Loop

```bash
cd ../alpenglow
./install.sh --check
```

Alpenglow consumes Soliloquy's `ui/desktop/build` output when composing an OS image.
