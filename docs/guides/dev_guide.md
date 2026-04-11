# Soliloquy Developer Guide

## 1. Project Vision
**Soliloquy** is a minimal, web-native operating system built on the **Zircon microkernel**. It aims to be what ChromeOS could have been: a pure web runtime without legacy baggage.

- **Kernel**: Zircon (Fuchsia's microkernel)
- **Userland**: Minimal drivers + Servo Browser Engine
- **UI**: Svelte-based Web UI (Plates)
- **Target Hardware (for now)**: Radxa Cubie A5E (Allwinner A527, ARM64)

## 2. Architecture Stack
The system is layered as follows:

1.  **Hardware**: Allwinner A527 (ARM64)
2.  **Kernel**: Zircon (handles scheduling, memory, IPC)
3.  **Drivers (DDK)**:
    -   Written in C++ (mostly) or Rust.
    -   Key drivers: UART, eMMC, Ethernet, AIC8800 WiFi, Mali GPU.
4.  **System Services**: Minimal services for network and graphics (Magma).
5.  **Runtime**: **Servo** (Rust-based browser engine).
    -   **JS Engine**: `rusty_v8` (V8 bindings for Rust).
    -   **Rendering**: WebRender (GPU accelerated).
6.  **Application**: The "Shell" is just a web page running in Servo.

## 3. Development Environment

### Supported Platforms
- **Linux**: Fedora, RHEL, Debian, Ubuntu (full source build supported)
- **macOS**: macOS 10.15+ with Homebrew (SDK-only setup supported; full source build requires remote Linux instance)

### Primary Build System
- **Build Tools**: `gn` (Generate Ninja) and `ninja`.
- **Language Toolchains**:
    -   **Rust**: Stable, with `aarch64-unknown-fuchsia` target.
    -   **C++**: Clang/LLVM (provided by Fuchsia tree).
    -   **Python**: 3.8+ for build scripts.

### macOS vs Linux Build Options

**Option 1: Full Source Build (Linux Only)**
- Requires a Linux machine or VM (Fedora/Debian/Ubuntu)
- Run `tools/soliloquy/setup.sh` to bootstrap the full Fuchsia source tree
- More flexibility for kernel and driver development
- Longer initial setup (~1-2 hours for initial clone and bootstrap)

**Option 2: SDK-Only Setup (Linux and macOS)**
- Works on macOS or Linux
- Run `tools/soliloquy/setup_sdk.sh` to download a pre-built Fuchsia SDK
- Faster setup (~5-10 minutes for SDK download)
- Suitable for application development and testing
- Avoids Google Source rate limiting issues

### macOS Remote Build Workflow

If you are developing on macOS and need to build the full source:
1. Set up a Fedora or Linux VM/container (locally or in the cloud)
2. Clone this repository on the Linux instance
3. Run `tools/soliloquy/setup.sh` on the Linux instance
4. Sync code changes to the Linux instance (e.g., via git, rsync, or SSH)
5. Run build commands on the Linux instance via SSH
6. Copy build artifacts back to macOS for testing (if needed)

### Directory Structure
```
//
├── boards/               # Board definitions
│   └── arm64/soliloquy/  # Radxa Cubie A5E config
├── drivers/              # Custom drivers
│   └── wifi/aic8800/     # Ported WiFi driver
├── tools/soliloquy/      # Helper scripts (build, setup, flash)
├── vendor/               # Third-party deps (Servo, etc.)
└── ... (Standard Fuchsia tree)
```

## 4. Workflows for AI Agents & Developers

### Setting Up

#### On Linux (Full Source Build)
1. **Bootstrap**: Run `./tools/soliloquy/setup.sh`
   - Installs dependencies (git, curl, unzip, build tools)
   - Clones the Fuchsia repository
   - Bootstraps the Fuchsia build system
   - Links Soliloquy sources into the Fuchsia tree
2. **Environment**: Always `source scripts/fx-env.sh` before running `fx` commands

#### On macOS (SDK-Only Setup)
1. **Prerequisites**: Install Homebrew if not already installed:
   ```bash
   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
   ```
2. **SDK Setup**: Run `./tools/soliloquy/setup_sdk.sh`
   - Detects macOS architecture (Intel x86_64 or Apple Silicon arm64)
   - Downloads the appropriate Fuchsia SDK bundle (mac-amd64 or mac-arm64)
   - Verifies curl and unzip are available
   - Extracts the SDK to `./sdk`
3. **Environment**: Source the environment helper script:
   ```bash
   source tools/soliloquy/env.sh
   ```
   This will automatically detect your OS and set:
   - `FUCHSIA_DIR` to the SDK or Fuchsia source directory
   - `CLANG_HOST_ARCH` to `mac-x64` (macOS) or `linux-x64` (Linux)
   - Add SDK tools to your PATH

#### SDK Version Pinning
To use a specific SDK version instead of "latest":
```bash
export FUCHSIA_SDK_VERSION=20231115.2  # Example version
./tools/soliloquy/setup_sdk.sh
```

### Building
Use the helper script:
```bash
./tools/soliloquy/build.sh
```
Or manually:
```bash
fx set minimal.arm64 --board soliloquy
fx build
```

### Remote Building (Fedora via SSH)
If you are on macOS and using the Fedora instance (`undivisible@fedora@orb`):
1.  Sync code to the remote instance.
2.  Run build commands via SSH.
3.  (Optional) Use `tools/soliloquy/ssh_build.sh` (if available).

### Adding a Driver

Soliloquy provides a Hardware Abstraction Layer (HAL) in `drivers/common/soliloquy_hal` that simplifies common driver tasks. All new drivers should use this HAL.

📖 **For detailed driver porting instructions, see the [Driver Porting Guide](docs/driver_porting.md)** - a comprehensive walkthrough of porting Linux drivers to Soliloquy, with API mappings, HAL usage examples, and common pitfall avoidance.

#### HAL Components

The `soliloquy_hal` library provides:

- **Firmware Loading** (`firmware.h`): Load and map firmware files
- **SDIO Helpers** (`sdio.h`): Simplified SDIO transaction helpers and firmware download
- **MMIO Access** (`mmio.h`): Register read/write with bit manipulation utilities
- **Clock/Reset Control** (`clock_reset.h`): Enable/disable clocks and manage resets

#### Creating a New Driver

1.  **Create directory**: `drivers/<category>/<name>/`

2.  **Implement driver class**:
    ```cpp
    #include "../../common/soliloquy_hal/firmware.h"
    #include "../../common/soliloquy_hal/mmio.h"
    // etc.

    class MyDriver : public ddk::Device<MyDriver> {
      // Use HAL helpers
      std::unique_ptr<soliloquy_hal::MmioHelper> mmio_helper_;
    };
    ```

3.  **Create `BUILD.gn`**:
    ```gn
    driver_module("my_driver") {
      sources = [ "my_driver.cc", "my_driver.h" ]
      deps = [
        "//drivers/common/soliloquy_hal",  # Include HAL
        "//src/lib/ddk",
        # ... other deps
      ]
    }
    ```

4.  **Register board device** in `boards/arm64/soliloquy/src/soliloquy-<device>.cc`:
    - Define MMIO/IRQ resources using `pbus_mmio_t`, `pbus_irq_t`
    - Call `pbus_.DeviceAdd()` in init function
    - Add init call to `Soliloquy::Start()` in `soliloquy.cc`

5.  **Add to board config**: Edit `boards/arm64/soliloquy/board_config.gni`:
    ```gn
    board_driver_package_labels = [
      "//drivers/<category>/<name>:<target>",
    ]
    ```

#### Example: GPIO Driver

See `drivers/gpio/soliloquy_gpio/` for a reference implementation that uses the HAL for MMIO access.

#### Example: WiFi Driver (AIC8800)

The AIC8800 driver (`drivers/wifi/aic8800/`) demonstrates HAL usage for:
- Firmware loading with `FirmwareLoader::LoadFirmware()`
- SDIO transactions with `SdioHelper`
- Firmware download to hardware

## 5. c2v Translation (Experimental)

Soliloquy includes experimental tooling for translating Zircon C subsystems to the V programming language using the c2v translator. This is part of a gradual migration effort to improve memory safety.

📖 **For complete c2v workflow documentation, see the [Zircon C-to-V Translation Guide](docs/zircon_c2v.md)** - comprehensive documentation on the translation tooling, workflow, and subsystem priority list.

### Quick Start

```bash
# Install V toolchain
./tools/soliloquy/c2v_pipeline.sh --bootstrap-only

# Translate a subsystem (dry-run)
./tools/soliloquy/c2v_pipeline.sh --subsystem kernel/lib/libc --dry-run

# Run smoke test
bazel build //:c2v_tooling_smoke
```

## 6. Testing

Soliloquy uses a comprehensive test framework with unit tests, integration tests, and code coverage.

### Running Tests

```bash
# Run all Rust tests
./tools/soliloquy/test.sh

# Run with coverage
./tools/soliloquy/test.sh --coverage

# Run C++ tests (requires Fuchsia source)
./tools/soliloquy/test.sh --fx-test
```

### Test Structure

- **Unit Tests**: Co-located with source code
- **Integration Tests**: In `src/shell/integration_tests.rs` and `src/shell/fidl_integration_tests.rs`
- **C++ Tests**: In `drivers/*/tests/` directories
- **Mock FIDL Servers**: In `test/support/` crate

See [Testing Guide](docs/testing.md) for detailed documentation.

## 7. Current Status & Roadmap
- [x] **Board Config**: Basic GN files for Soliloquy board created.
- [x] **Driver HAL**: Common hardware abstraction layer (`drivers/common/soliloquy_hal`) for MMIO, SDIO, firmware loading, and clock/reset control.
- [x] **WiFi Driver**: AIC8800D80 driver refactored to use HAL.
- [x] **GPIO Driver**: Generic GPIO driver using HAL as reference implementation.
- [x] **Test Framework**: Unified test support crate with mock FIDL servers and coverage tools.
- [x] **c2v Tooling**: Experimental C-to-V translation pipeline for Zircon subsystems.
- [ ] **Servo Integration**: Needs platform abstraction layer for Zircon.
- [ ] **GPU Driver**: Mali-G57 integration needed (Magma).

## 8. Key Constraints
- **No POSIX**: Do not assume standard libc/POSIX availability in kernel/driver space.
- **Async First**: Use Zircon's async loop and FIDL for IPC.
- **Web Only**: No standalone terminal app, no X11/Wayland. The "display" is a full-screen browser.
