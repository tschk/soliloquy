# Soliloquy Build Guide

This page documents the active local build paths for the Alpine appliance, Servo desktop bundle, `sold` bridge, and RV8/Servo linkage work.

## Active Build Paths

| Area | Command | Notes |
|------|---------|-------|
| Desktop UI bundle | `./tools/soliloquy/build_ui.sh` | Builds the static bundle loaded by Servo |
| UI development server | `./tools/soliloquy/dev_ui.sh` | Runs the SvelteKit dev server through Bun |
| Local runtime | `./tools/soliloquy/start.sh` | Starts the local runtime path |
| Servo/RV8 bridge checks | `./tools/rv8_servo_test.sh bridge` | Runs targeted bridge validation |
| Rust workspace checks | `cargo test` | Runs Rust tests for the workspace |

## Desktop UI Bundle

```bash
./tools/soliloquy/build_ui.sh
```

Use this before staging the desktop surface into the appliance image or testing the static bundle with Servo.

The direct UI command is:

```bash
cd ui/desktop
bun run build
```

## UI Development

```bash
./tools/soliloquy/dev_ui.sh
```

The dev server is the fastest path for desktop UI iteration.

## Runtime Startup

```bash
./tools/soliloquy/start.sh
```

This starts the local Soliloquy runtime path for the desktop appliance workflow.

## Servo/RV8 Bridge Checks

```bash
./tools/rv8_servo_test.sh bridge
```

This is the targeted check for the Servo bridge and RV8 linkage surface.

## Rust Checks

```bash
cargo test
```

Use targeted Cargo package checks when working in a specific Rust component.

## Appliance Image Work

The active appliance image path lives under `system/alpine`. That tree owns Alpine package staging, service files, and image assembly details. Keep image-specific edits in that tree and use the local runtime commands above for UI and bridge validation.

## macOS Packages

Use Wax for host package installation:

```bash
wax install bazelisk
```

## See Also

- [V0 Architecture](./v0-architecture.md)
- [Appliance System Architecture](./architecture/appliance-system.md)
- [RV8 Linkage Roadmap](./rv8_linkage_roadmap.md)
- [Tools Reference](./guides/tools_reference.md)
