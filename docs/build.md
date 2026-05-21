# Build

Soliloquy uses Cargo for Rust, Bun for the Svelte UI, and `system/alpine/scripts` for appliance staging.

## Rust

```bash
cargo build
cargo test -p sold
cargo test -p soliloquy-shell --lib
```

The sibling RV8 repo is checked directly:

```bash
cargo test --manifest-path ../rv8/Cargo.toml
```

## UI

```bash
cd ui/desktop
bun install
bun run check
bun run build
```

## macOS Desktop Smoke

```bash
./tools/soliloquy/smoke_macos.sh
```

This checks the non-GPUI desktop binary, checks the Crepuscularity GPUI desktop binary, and dry-runs the macOS launch contract. The supported macOS desktop path is Crepuscularity chrome plus Servo with `--no-browser-chrome`; it must not start `sold` or load the Svelte appliance chrome. `sold` and the Svelte desktop bundle belong to the Alpine appliance path.

To launch the macOS browser:

```bash
./tools/soliloquy/start_macos.sh
```

## Local Runtime

```bash
./tools/soliloquy/start.sh
```

The appliance session defaults to `SOL_SERVO_NO_BROWSER_CHROME=1`. Keep Soliloquy's Svelte appliance shell as the single visible browser chrome; Servo should render page content only. Browser UI modes are available from the appliance chrome: Zen sidebar, compact, split columns, split rows, and grid.

## Alpine Appliance

```bash
./system/alpine/scripts/setup-host.sh
./system/alpine/scripts/qemu-v0.sh
```

Validate the custom appliance kernel profile without a full kernel build:

```bash
./system/alpine/kernel/validate-kernel-config.sh
./system/alpine/kernel/validate-kernel-config.sh /path/to/linux/.config
```

The custom kernel packaging scaffold lives under `system/alpine/kernel`. Its fragment keeps cgroup v2, zram, virtio, DRM/KMS, seccomp, Landlock, fq, and BBR enabled while rejecting unused desktop/server drivers, filesystems, and network protocols.
