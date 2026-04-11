# Soliloquy v0

Soliloquy v0 is a **browser-only immutable Linux appliance** on Alpine Linux:

- Alpine base (musl + minimal userland)
- wlroots kiosk compositor (`cage`)
- Servo as the fullscreen browser surface
- No login manager; boot directly to web UI
- Read-only root filesystem with writable browser profile/cache/state
- Servo fork built in-tree and launched fullscreen at boot
- Local `sold` bridge serves the browser bundle and terminal PTY

This reset replaces the previous multi-target architecture with a focused, shippable v0.

## Project layout

```text
system/
  alpine/
    openrc/              # OpenRC service units (browser session launcher)
    scripts/             # Session/startup/image scripts for cage + servo
```

## Quick start (dev host)

Build+boot through the Alpine/QEMU flow in `system/alpine`.

## Alpine tree-in-repo

Alpine image/bootstrap assets now live under `system/alpine`:

- `scripts/setup-host.sh` - installs host deps and sets up Wax from GitHub
  - on macOS, it also installs Servo's GStreamer/framework dependencies when missing
- `scripts/build-rootfs.sh` - builds or reuses a minimal Alpine rootfs artifact
- `scripts/ensure-servo-fork.sh` - ensures your in-tree Servo fork exists and is built
- `scripts/configure-rootfs.sh` - applies Soliloquy overlay + OpenRC services
- `scripts/stage-soliloquy-artifacts.sh` - stages Servo, sold, and the browser bundle into rootfs
- `scripts/qemu-v0.sh` - full build-and-run QEMU flow

## Boot target design

1. OpenRC starts `seatd`.
2. OpenRC starts `sold`.
3. OpenRC starts `sol-session`.
4. `sol-session` launches `cage` and runs Servo fullscreen.
