# AIC8800 WiFi Driver for Fuchsia

This directory contains the AIC8800 WiFi driver for the Soliloquy A527 board running Fuchsia.

## Overview

The AIC8800 is a WiFi 4 (802.11n) chipset connected via SDIO. This driver implements the WLANPHY protocol to provide WiFi functionality.

## Directory Structure

```
drivers/wifi/aic8800/
├── aic8800.h              # Main driver header
├── aic8800.cc             # Main driver implementation
├── BUILD.gn               # Build configuration (GN)
├── BUILD.bazel            # Build configuration (Bazel)
├── README.md              # This file
├── firmware/              # Firmware binaries
│   └── aic8800DC/         # AIC8800DC firmware files
├── linux_reference/       # Linux driver mapping documentation
│   └── README.md          # Detailed Linux→Fuchsia mapping
├── mock/                  # Rust mock utilities for testing
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── sdio.rs        # Mock SDIO device
│       ├── firmware.rs    # Mock firmware loader
│       └── register.rs    # Register definitions and helpers
└── tests/                 # Rust integration tests
    ├── Cargo.toml
    ├── lib.rs
    └── integration_test.rs
```

## Building

### C++ Driver

Build the driver using GN:

```bash
fx build //drivers/wifi/aic8800:aic8800
```

Or with Bazel:

```bash
bazel build //drivers/wifi/aic8800:aic8800
```

Or build everything:

```bash
fx build
```

### Rust Mock Utilities and Tests

The mock utilities and tests can be run independently:

```bash
cd drivers/wifi/aic8800/mock
cargo test --target x86_64-unknown-linux-gnu

cd ../tests
cargo test --target x86_64-unknown-linux-gnu
```

## Firmware Requirements

The driver requires firmware binaries to be present in the `firmware/` directory. The main firmware file is:

- **fmacfw_8800d80.bin**: Main firmware for AIC8800DC variant

Additional firmware files may be required depending on the chip variant:

- **fmacfw_patch_8800dc_u02.bin**: Firmware patch
- **fmacfw_patch_tbl_8800dc_u02.bin**: Patch table
- **fmacfw_calib_8800dc_u02.bin**: Calibration data

## Hardware Setup

### SDIO Connection

The AIC8800 connects to the A527 SoC via SDIO:

- **SDIO Function**: Function 1
- **Block Size**: 512 bytes (default)
- **Clock**: Up to 50 MHz
- **Data Width**: 4-bit

### Register Layout

Key registers (all 32-bit, little-endian):

| Address | Register | Description |
|---------|----------|-------------|
| 0x00000000 | REG_CHIP_ID | Chip identification |
| 0x00000004 | REG_CHIP_REV | Chip revision |
| 0x00000008 | REG_FW_STATUS | Firmware status |
| 0x0000000C | REG_HOST_CTRL | Host control |
| 0x00000010 | REG_INT_STATUS | Interrupt status |
| 0x00000014 | REG_INT_MASK | Interrupt mask |
| 0x00000002 | REG_BYTEMODE_LEN | RX length in byte mode |
| 0x00000005 | REG_SLEEP_CTRL | Sleep control |
| 0x00000009 | REG_WAKEUP | Wakeup trigger |
| 0x0000000A | REG_FLOW_CTRL | Flow control/buffer status |
| 0x00100000 | REG_FW_DOWNLOAD_ADDR | Firmware download base |
| 0x00120000 | RAM_FMAC_FW_ADDR_U02 | Firmware RAM address (U02) |

See `mock/src/register.rs` for complete register definitions.

## Initialization Sequence

1. **SDIO Init**: Parent SDIO device is initialized by platform driver
2. **Block Size**: Set SDIO block size to 512 bytes
3. **Chip ID**: Read chip ID and verify (0x88000001 for AIC8800DC)
4. **Reset**: Assert and deassert reset via HOST_CTRL register
5. **Firmware Load**: Load firmware from package
6. **Firmware Download**: Download firmware to device RAM (0x00120000) via SDIO
7. **Patch Configuration**: Program patch tables with magic numbers and configuration
8. **Wait Ready**: Poll FW_STATUS register for READY state
9. **Enable**: Enable chip via HOST_CTRL register

## SDIO Data Path

### Flow Control

The driver implements flow control based on the Linux reference implementation:

```cpp
// Check available buffers before transmitting
uint8_t available_buffers;
SdioFlowControl(&available_buffers);  // Polls REG_FLOW_CTRL with timeout
```

Flow control behavior:
- Reads `REG_FLOW_CTRL` (0x0A) and masks with 0x7F
- Retries up to 50 times with adaptive delays (200µs → 1ms → 10ms)
- Returns number of available 1536-byte buffers

### TX Operations

```cpp
// Transmit data with flow control
const uint8_t tx_data[256];
SdioTx(tx_data, sizeof(tx_data), 7);  // Function 7 for TX

// Flow control is checked automatically
// Length is block-aligned to 512 bytes
```

### RX Operations

```cpp
// Receive data
uint8_t rx_data[1024];
SdioRx(rx_data, sizeof(rx_data), 8);  // Function 8 for RX

// Length is block-aligned to 512 bytes
```

### Byte Operations

```cpp
// Read a single byte (for registers)
uint8_t value;
sdio_helper_.ReadByte(address, &value);

// Write a single byte (for registers)
sdio_helper_.WriteByte(address, value);
```

### Block Operations

```cpp
// Read multiple blocks (for data)
uint8_t buffer[1024];
sdio_helper_.ReadMultiBlock(address, buffer, sizeof(buffer));

// Write multiple blocks (for data)
uint8_t buffer[1024];
sdio_helper_.WriteMultiBlock(address, buffer, sizeof(buffer));
```

### Firmware Download

```cpp
// Load firmware from package
zx::vmo fw_vmo;
size_t fw_size;
FirmwareLoader::LoadFirmware(parent(), "fmacfw_8800d80.bin", &fw_vmo, &fw_size);

// Download to device RAM
sdio_helper_.DownloadFirmware(fw_vmo, fw_size, 0x00120000);
```

### Patch Configuration

After firmware download, patch tables are configured:

```cpp
ConfigurePatchTables();  // Programs patch magic numbers and entries
```

The patch table structure (from Linux reference):
- Magic numbers: 0x48435450 ("PTCH"), 0x50544348 ("HCTP")
- Patch entries at 0x001D7000:
  - {config_base + 0x00b4, 0xf3010000} - Enable 2.4GHz only
  - {config_base + 0x0170, 0x0001000A} - RX aggregation counter

## WLANPHY Protocol Implementation

The driver implements the `WlanphyImpl` protocol:

### WlanphyImplQuery

Returns device capabilities:
- Supported bands (2.4 GHz)
- MAC modes (STA, AP)
- PHY types (802.11n)
- Supported channels

### WlanphyImplCreateIface

Creates a WiFi interface (STA or AP mode).

### WlanphyImplDestroyIface

Destroys a WiFi interface.

### WlanphyImplSetCountry / WlanphyImplGetCountry

Country code management for regulatory compliance.

## Mock Utilities (Rust)

The mock utilities provide a testable implementation of the SDIO and firmware loading subsystems.

### MockSdioDevice

Simulates an SDIO device with:
- Memory-mapped register space
- Byte and block read/write operations
- Firmware download simulation
- Transaction history recording
- Error injection for testing

Example usage:

```rust
use aic8800_mock::MockSdioDevice;

let sdio = MockSdioDevice::new();
sdio.initialize()?;
sdio.set_block_size(512)?;

// Write and read
sdio.write_byte(0x1000, 0x42)?;
let value = sdio.read_byte(0x1000)?;

// Download firmware
let firmware = vec![0xAA; 4096];
sdio.download_firmware(0x00100000, &firmware)?;

// Verify
assert!(sdio.verify_firmware_at(0x00100000, &firmware));
```

### MockFirmwareLoader

Simulates firmware loading:
- Multiple firmware files
- Load counting
- Test firmware generation

Example usage:

```rust
use aic8800_mock::MockFirmwareLoader;

let loader = MockFirmwareLoader::new();

// Add firmware
let firmware = MockFirmwareLoader::create_aic8800_firmware();
loader.add_firmware("fmacfw_8800d80.bin", firmware);

// Load firmware
let loaded = loader.load_firmware("fmacfw_8800d80.bin")?;
```

### Aic8800Registers

Register constants and helper functions:

```rust
use aic8800_mock::Aic8800Registers;

// Register addresses
let chip_id_addr = Aic8800Registers::REG_CHIP_ID;
let ctrl_addr = Aic8800Registers::REG_HOST_CTRL;

// Control bits
let reset_bit = Aic8800Registers::HOST_CTRL_RESET;
let enable_bit = Aic8800Registers::HOST_CTRL_ENABLE;

// Helper functions
if Aic8800Registers::is_valid_chip_id(chip_id) {
    println!("Chip: {}", Aic8800Registers::chip_id_to_string(chip_id));
}
```

## Testing

### Rust Unit Tests

Each mock module has comprehensive unit tests:

```bash
cd drivers/wifi/aic8800/mock
cargo test --target x86_64-unknown-linux-gnu
```

Tests cover:
- SDIO initialization and operations
- Firmware loading and management
- Register operations
- Error conditions

### Rust Integration Tests

Integration tests verify the complete flow:

```bash
cd drivers/wifi/aic8800/tests
cargo test --target x86_64-unknown-linux-gnu
```

Tests cover:
- Full initialization sequence
- Firmware download and verification
- Register sequences
- Error recovery
- Transaction history

### C++ Tests (Future Work)

C++ unit tests for the driver implementation will be added:

```bash
fx test aic8800_tests
```

## Linux Reference

The Linux driver provides the reference implementation. See `linux_reference/README.md` for detailed mapping of Linux driver functions to Fuchsia equivalents.

Key mappings:
- Linux `sdio_readb/writeb` → Fuchsia `SdioHelper::ReadByte/WriteByte`
- Linux `sdio_memcpy_toio` → Fuchsia `SdioHelper::WriteMultiBlock`
- Linux `request_firmware` → Fuchsia `FirmwareLoader::LoadFirmware`

## Current Status

### Implemented ✅

- [x] Basic driver structure (DDK integration)
- [x] SDIO helper library (`soliloquy_hal::SdioHelper`)
- [x] Firmware loader (`soliloquy_hal::FirmwareLoader`)
- [x] Firmware download via SDIO
- [x] Rust mock utilities
  - [x] MockSdioDevice
  - [x] MockFirmwareLoader
  - [x] Register definitions
- [x] Rust unit tests (19 tests, all passing)
- [x] Rust integration tests (10 tests, all passing)
- [x] Linux reference documentation

### In Progress 🚧

- [ ] Chip ID verification
- [ ] Firmware status polling
- [ ] WLANPHY protocol implementation
- [ ] Interrupt handling

### Future Work 📋

- [ ] Complete WLANPHY methods
- [ ] Interface management (STA/AP)
- [ ] TX/RX data path
- [ ] Power management
- [ ] Country code support
- [ ] C++ unit tests
- [ ] Hardware testing on A527 board

## Debugging

### Enable Logging

In C++ driver:

```cpp
zxlogf(INFO, "aic8800: Message");
zxlogf(ERROR, "aic8800: Error: %s", zx_status_get_string(status));
```

In Rust tests:

```bash
RUST_LOG=debug cargo test --target x86_64-unknown-linux-gnu
```

### SDIO Transaction History

The mock SDIO device records all transactions:

```rust
let transactions = sdio.get_transactions();
for tx in transactions {
    println!("Addr: 0x{:08x}, Write: {}, Data: {:02x?}", 
             tx.address, tx.is_write, tx.data);
}
```

### Memory Snapshot

Dump SDIO memory for debugging:

```rust
let memory = sdio.get_memory_snapshot();
for (addr, value) in memory.iter() {
    println!("0x{:08x}: 0x{:02x}", addr, value);
}
```

## Contributing

When modifying the driver:

1. Update register constants in `mock/src/register.rs`
2. Add corresponding C++ constants in `aic8800.h`
3. Update Linux mapping in `linux_reference/README.md`
4. Add tests for new functionality
5. Run all tests before committing

## References

- [Fuchsia SDIO Protocol](https://fuchsia.dev/reference/fidl/fuchsia.hardware.sdio)
- [Fuchsia WLANPHY Protocol](https://fuchsia.dev/reference/fidl/fuchsia.hardware.wlanphyimpl)
- [Linux AIC8800 Driver](../../vendor/aic8800-linux/)
- [Soliloquy HAL Documentation](../../common/soliloquy_hal/README.md)
