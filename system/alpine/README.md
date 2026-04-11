# Alpine tree (in-repo)

This directory carries the Alpine-specific boot/runtime layout for the Soliloquy browser appliance:

- `packages-v0.txt` - minimal runtime package manifest
- `packages-v0-dev.txt` - optional dev extras (terminal-oriented)
- `rootfs-overlay/` - files copied directly into Alpine rootfs
- `openrc/` - OpenRC service units for the browser session
- `scripts/` - helper scripts for rootfs assembly and image prep
- `docker/` - rootfs builder implementation for reproducible output

## Expected v0 boot

1. OpenRC starts core services (`seatd`).
2. OpenRC starts `sold` to serve the local browser UI and PTY bridge.
3. `sol-session` starts the browser session.
4. `cage` launches Servo fullscreen.
5. Servo opens the local Soliloquy browser surface.

The root filesystem is treated as immutable at runtime; browser profile, cache, downloads, logs, and terminal state are the writable areas.

The first-boot hook prunes nonessential default services (logging daemons, avahi, cron, bluetooth, etc.) to keep startup and memory overhead low.

## Build rootfs with Docker

```sh
./system/alpine/scripts/setup-host.sh
./system/alpine/scripts/build-rootfs.sh
```

`setup-host.sh` checks for Wax first, then bootstraps the minimal host tools plus Servo's macOS GStreamer/framework dependencies when needed. The rootfs builder reuses the staged Alpine artifact when it is already present, and only regenerates it when the cache is cold or `FORCE_ROOTFS_REBUILD=1`.

Output:

- `build/alpine/rootfs.tar.gz`
- `build/alpine/rootfs/` (extracted rootfs)

## Full QEMU flow

```sh
./system/alpine/scripts/qemu-v0.sh
```

Build-only validation without starting the VM:

```sh
QEMU_RUN=0 ./system/alpine/scripts/qemu-v0.sh
```

For architecture selection:

```sh
QEMU_ARCH=x86_64 ./system/alpine/scripts/qemu-v0.sh
```

Servo fork requirements (in-tree):

```sh
SERVO_FORK_URL=https://github.com/<org-or-user>/servo.git QEMU_ARCH=x86_64 ./system/alpine/scripts/qemu-v0.sh
```

This clones your fork into `third_party/servo` (if missing), builds it, and stages:

- `third_party/servo/target/release/servoshell` -> `/usr/local/bin/servo`
- `target/release/sold` -> `/usr/local/bin/sold`
- `ui/desktop/build` -> `/usr/local/share/soliloquy/ui`

Important: the staged `servo` and `sold` binaries must be Linux ELF binaries for the selected `QEMU_ARCH`.
Building on macOS produces Mach-O binaries, which cannot run inside the Alpine Linux VM.
The staging script now fails fast when binary formats do not match.

Force a clean Servo rebuild during QEMU flow:

```sh
SERVO_FORCE_REBUILD=1 QEMU_ARCH=x86_64 ./system/alpine/scripts/qemu-v0.sh
```

Manual steps:

```sh
./system/alpine/scripts/build-rootfs.sh
./system/alpine/scripts/ensure-servo-fork.sh
./system/alpine/scripts/stage-soliloquy-artifacts.sh build/alpine/rootfs
./system/alpine/scripts/fetch-qemu-kernel.sh build/alpine/qemu
./system/alpine/scripts/build-qemu-initramfs.sh build/alpine/rootfs build/alpine/qemu/rootfs.cpio.gz
./system/alpine/scripts/run-qemu.sh build/alpine/qemu
```
