# Soliloquy OS Product Configuration Summary

This document summarizes the Fuchsia product configuration created for Soliloquy OS targeting the Radxa Cubie A5E (Allwinner A527 ARM64).

## Overview

Soliloquy OS is configured as a minimal, web-UI-only Fuchsia product optimized for the Radxa Cubie A5E SBC. It uses the Servo browser engine with V8 JavaScript runtime and Svelte 5 UI framework.

## Directory Structure

```
products/
├── soliloquy.arm64.gni      # Board/product configuration
├── soliloquy_packages.gni   # Package definitions
└── soliloquy/
    └── BUILD.gn             # Product assembly configuration

boards/arm64/
├── soliloquy/               # Main board configuration
│   ├── BUILD.gn             # Driver definitions
│   ├── board.gni            # Hardware topology
│   ├── board_config.gni     # Build configuration
│   ├── meta/                # Component manifests
│   │   ├── soliloquy-platform.cml
│   │   ├── soliloquy-gpio.cml
│   │   ├── soliloquy-sdio.cml
│   │   ├── soliloquy-ethernet.cml
│   │   ├── soliloquy-display.cml
│   │   ├── soliloquy-mmc.cml
│   │   ├── soliloquy-hid.cml
│   │   └── bind/            # Driver bind rules
│   ├── src/                 # Driver source files
│   │   ├── soliloquy-display.cc
│   │   ├── soliloquy-hid.cc
│   │   └── soliloquy-mmc.cc
│   └── *-info.json          # Driver metadata
└── soliloquy-qemu/          # QEMU configuration
    ├── BUILD.gn
    └── board.gni
```

## Products

### soliloquy (Full UI)
- Complete web-UI product with Servo + V8 + Svelte
- Display, input, networking support
- Software rendering (no GPU driver)
- Ideal for development and production

### soliloquy_headless (No Display)
- Backend-only mode
- API server at port 3030
- Networking enabled
- Ideal for embedded use

### soliloquy_qemu (Testing)
- QEMU ARM64 virt machine
- VirtIO drivers
- Network forwarding
- Ideal for CI/CD and testing

### soliloquy_with_tests (Development)
- Includes test packages
- Debug logging enabled
- Bootstrap testing framework

## Drivers

| Driver | Protocol | Description |
|--------|----------|-------------|
| soliloquy-platform | fuchsia.hardware.platform.bus | Main board driver |
| soliloquy-gpio | fuchsia.hardware.gpio | Allwinner GPIO controller |
| soliloquy-sdio | fuchsia.hardware.sdio | SDIO controller (WiFi, SD) |
| soliloquy-ethernet | fuchsia.hardware.network | Allwinner EMAC |
| soliloquy-display | fuchsia.hardware.display.controller | DE3.0 display engine |
| soliloquy-mmc | fuchsia.hardware.block | eMMC/SD storage |
| soliloquy-hid | fuchsia.input.report | Touch/button input |
| aic8800 | fuchsia.wlan.softmac | AIC8800D80 WiFi |

## Quick Start

### Build Commands

```bash
# Full product
fx set soliloquy.arm64 --product soliloquy

# Headless
fx set soliloquy.arm64 --product soliloquy_headless

# QEMU testing
fx set soliloquy.qemu-arm64 --product soliloquy_qemu

# Build
fx build

# Flash to hardware
fx flash --pave

# QEMU
fx qemu -N
```

### Key Configuration Files

| File | Purpose |
|------|---------|
| `products/soliloquy.arm64.gni` | Target CPU, partitions, storage limits |
| `products/soliloquy_packages.gni` | Package lists for each product |
| `products/soliloquy/BUILD.gn` | Product assembly and platform config |
| `boards/arm64/soliloquy/board.gni` | Hardware topology, MMIO, IRQs |
| `boards/arm64/soliloquy/board_config.gni` | Build options, feature flags |

## Hardware Specifications

- **SoC**: Allwinner A527 (sun55i)
- **CPU**: 4x ARM Cortex-A55 @ 1.8GHz
- **GPU**: Mali-G57 MC1 (not yet enabled)
- **RAM**: 2-4GB LPDDR4X
- **Storage**: eMMC + microSD
- **Network**: Gigabit Ethernet (RTL8211F PHY) + WiFi (AIC8800D80)
- **Display**: HDMI + DSI via DE3.0

## Platform Configuration

```gni
# Key platform settings
feature_set_level = "utility"  # Above bootstrap, below standard
use_flatland = true            # Modern compositor
use_software_rendering = true  # No GPU driver yet
data_filesystem_format = "f2fs"  # Flash-friendly filesystem
```

## Next Steps

1. **GPU Driver**: Port Mali-G57 driver for hardware acceleration
2. **Verified Boot**: Enable AVB chain of trust
3. **OTA Updates**: Configure Omaha update server
4. **Audio**: Add HDMI audio support
5. **Camera**: Add CSI camera driver

## Documentation

- [BUILD_AND_BOOT.md](BUILD_AND_BOOT.md) - Detailed build instructions
- [FLASH_COMMANDS.md](FLASH_COMMANDS.md) - Flash command reference
- [DRIVER_STATUS.md](DRIVER_STATUS.md) - Driver porting progress
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture
