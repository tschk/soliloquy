# Soliloquy Tools Reference

Reference for the active tools and scripts in the Soliloquy project.

## Runtime Tools

### `tools/soliloquy/start.sh`
Start the local Soliloquy runtime.

```bash
./tools/soliloquy/start.sh
```

This starts the local backend and UI path used by the desktop appliance workflow.

### `tools/soliloquy/dev_ui.sh`
Start the desktop UI development server.

```bash
./tools/soliloquy/dev_ui.sh
```

This runs the SvelteKit development server through Bun for the Servo desktop surface.

### `tools/soliloquy/build_ui.sh`
Build the static desktop bundle loaded by Servo.

```bash
./tools/soliloquy/build_ui.sh
```

The generated bundle can be staged into the appliance image or loaded by Servo during local runtime work.

## Bridge Checks

### `tools/rv8_servo_test.sh`
Run targeted Servo/RV8 bridge validation with the local toolchain environment.

```bash
./tools/rv8_servo_test.sh bridge
```

Checks include Servo bridge tests, bridge schema parser coverage, and the local toolchain setup needed for this narrow path.

## Hardware Tools

### `tools/soliloquy/debug.sh`
Connect to the target device serial console for debugging.

```bash
./tools/soliloquy/debug.sh
```

Use this when the target device is connected through the configured serial path.

## Tool Troubleshooting

### "Command not found"
Check the script exists in the active tools tree:

```bash
find tools -maxdepth 3 -type f -print
```

### UI build failures
Run UI commands through Bun:

```bash
cd ui/desktop
bun run build
```

### Missing macOS packages
Install host packages with Wax:

```bash
wax install bazelisk
```

## See Also

- [Build Guide](../build.md) - Build system documentation
- [Developer Guide](./dev_guide.md) - Development workflow
- [RV8 Linkage Roadmap](../rv8_linkage_roadmap.md) - Servo/RV8 bridge status
