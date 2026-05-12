# Tools Reference

## Runtime

- `tools/soliloquy/start.sh` starts `sold` and the UI dev server.
- `tools/soliloquy/build_ui.sh` builds the Svelte bundle with Bun.
- `tools/soliloquy/dev_ui.sh` starts the UI dev server with Bun.
- `tools/soliloquy/smoke_macos.sh` checks the Svelte bundle, non-GPUI macOS desktop build, and Servo launch contract without launching a persistent GUI.
- `tools/soliloquy/start_macos.sh` launches `sold` and Servo on macOS with Servo's built-in chrome disabled.

## Bridge

- `tools/rv8_servo_test.sh bridge` runs bridge checks.
- `tools/rv8_servo_test.sh v8` runs V8-path checks when the Servo checkout is available.

## Appliance

- `system/alpine/scripts/setup-host.sh` prepares host tools.
- `system/alpine/scripts/qemu-v0.sh` boots the appliance path.
