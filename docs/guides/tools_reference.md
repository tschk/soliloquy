# Tools Reference

## Runtime

- `tools/soliloquy/start.sh` starts `sold` and the UI dev server.
- `tools/soliloquy/build_ui.sh` builds the Svelte bundle with Bun.
- `tools/soliloquy/dev_ui.sh` starts the UI dev server with Bun.
- `tools/soliloquy/smoke_macos.sh` checks the non-GPUI and Crepuscularity GPUI macOS desktop builds and dry-runs the launch contract without starting `sold` or launching a persistent GUI.
- `tools/soliloquy/start_macos.sh` starts or reuses `sold` for local runtime APIs, launches Crepuscularity GPUI chrome on macOS, and starts Servo with Servo's built-in chrome disabled.

## Bridge

- `tools/rv8_servo_test.sh bridge` runs bridge checks.
- `tools/rv8_servo_test.sh v8` runs V8-path checks when the Servo checkout is available.

## Appliance

- `../alpenglow/install.sh --check` validates the installable OS layer.
- `../alpenglow/system/alpine/scripts/qemu-v0.sh` runs the current reference OS image flow.
