# Soliloquy Developer Guide

## 1. Project Vision
**Soliloquy** is a minimal, web-native operating system appliance built on **Alpine Linux**. It aims to be what ChromeOS could have been: a pure web runtime without legacy baggage.

- **Base system**: Alpine Linux with OpenRC
- **Userland**: Servo Browser Engine + `sold` local system bridge
- **UI**: Svelte-based Web UI (Plates)
- **Target Hardware (for now)**: Radxa Cubie A5E (Allwinner A527, ARM64)

## 2. Architecture Stack
The system is layered as follows:

1.  **Hardware**: Allwinner A527 (ARM64)
2.  **Base OS**: Alpine Linux with OpenRC service startup
3.  **Drivers**:
    -   Written in Rust or native Linux driver stacks where available.
    -   Key targets: UART, eMMC, Ethernet, AIC8800 WiFi, Mali GPU.
4.  **System Services**: Minimal services for network, display, and local system control through `sold`.
5.  **Runtime**: **Servo** (Rust-based browser engine).
    -   **JS Engine**: `rusty_v8` (V8 bindings for Rust).
    -   **Rendering**: WebRender (GPU accelerated).
6.  **Application**: The "Shell" is just a web page running in Servo.

## 3. Development Environment

### Supported Platforms
- **Linux**: Fedora, RHEL, Debian, Ubuntu (full source build supported)
- **macOS**: macOS 10.15+ with Wax for package installation

### Primary Build System
- **Build Tools**: Alpine image scripts, Cargo, Bazel, and Bun for UI work.
- **Language Toolchains**:
    -   **Rust**: Stable, with `aarch64-unknown-fuchsia` target.
    -   **C++**: Clang/LLVM where native components require it.
    -   **Python**: 3.8+ for build scripts.

### macOS vs Linux Build Options

**Option 1: Appliance Image Work (Linux Only)**
- Requires a Linux machine or VM (Fedora/Debian/Ubuntu/Alpine)
- Use the Alpine image tooling under `system/alpine`
- More flexibility for service, boot, and hardware integration work

**Option 2: Runtime and UI Work (Linux and macOS)**
- Works on macOS or Linux
- Use Cargo, Bun, and the local tool scripts
- Suitable for Servo/RV8 bridge work, `sold` API work, and desktop UI development

### macOS Remote Build Workflow

If you are developing on macOS and need to build the appliance image:
1. Set up a Fedora or Linux VM/container (locally or in the cloud)
2. Clone this repository on the Linux instance
3. Run the Alpine image workflow on the Linux instance
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
├── third_party/          # Third-party deps (Servo, etc.)
├── sold/                 # Local system bridge
├── system/alpine/        # Appliance image and service staging
└── ...
```

## 4. Workflows for AI Agents & Developers

### Setting Up

#### On Linux (Appliance Work)
1. **Bootstrap dependencies**: Install native build tools and image tooling for the target distro.
2. **Image workflow**: Use the scripts and manifests under `system/alpine`.
3. **Runtime workflow**: Use Cargo for Rust checks and Bun for UI checks.

#### On macOS (Runtime Setup)
1. **Prerequisites**: Install Wax if not already installed:
   ```bash
   wax --version
   ```
2. **Runtime setup**: Install native build tools with Wax as needed.
3. **Runtime workflow**: Use Cargo for Rust checks and Bun for UI checks.

### Building
Use the helper script:
```bash
./tools/soliloquy/build_ui.sh
```
Or manually:
```bash
cd ui/desktop
bun run build
```

### Remote Building (Fedora via SSH)
If you are on macOS and using the Fedora instance (`undivisible@fedora@orb`):
1.  Sync code to the remote instance.
2.  Run build commands via SSH.
3.  Use SSH or rsync directly for appliance-image builds on the Linux host.

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

## 5. Runtime Integration

Soliloquy currently centers on the Alpine appliance path, the fullscreen Servo surface, the `sold` local bridge, and the RV8/Servo linkage work.

### Quick Start

```bash
# Start the desktop UI development server
./tools/soliloquy/dev_ui.sh

# Build the static bundle loaded by Servo
./tools/soliloquy/build_ui.sh

# Run targeted Servo/RV8 bridge checks
./tools/rv8_servo_test.sh bridge
```

## 6. Testing

Soliloquy uses a comprehensive test framework with unit tests, integration tests, and code coverage.

### Running Tests

```bash
# Run Rust tests
cargo test

# Run targeted Servo/RV8 bridge checks
./tools/rv8_servo_test.sh bridge

# Run UI checks from the desktop workspace
cd ui/desktop
bun run build
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
- [x] **Alpine Appliance**: Base image and service staging path.
- [x] **sold Bridge**: Local system bridge for authenticated system and terminal APIs.
- [ ] **Servo/RV8 Integration**: Needs broader bridge coverage for page-script ownership.
- [ ] **GPU Driver**: Mali-G57 integration needed (Magma).

## 8. Key Constraints
- **No POSIX**: Do not assume standard libc/POSIX availability in kernel/driver space.
- **Async First**: Use async service boundaries and narrow local bridge APIs.
- **Web Only**: No standalone terminal app, no X11/Wayland. The "display" is a full-screen browser.
