# Soliloquy Hardware Abstraction Layer (HAL)

The Soliloquy HAL provides reusable components for hardware drivers on the A527 board. This library abstracts common hardware operations to reduce boilerplate and improve code reuse across drivers.

## Components

### Firmware Loader (`firmware.h`)

Simplifies loading firmware from the system and mapping it into memory.

**Usage:**
```cpp
#include "../../common/soliloquy_hal/firmware.h"

zx::vmo fw_vmo;
size_t fw_size;
zx_status_t status = soliloquy_hal::FirmwareLoader::LoadFirmware(
    parent(), "my_firmware.bin", &fw_vmo, &fw_size);
```

### SDIO Helper (`sdio.h`)

Provides wrappers around SDIO protocol for byte and block operations.

**Usage:**
```cpp
#include "../../common/soliloquy_hal/sdio.h"

ddk::SdioProtocolClient sdio(parent);
soliloquy_hal::SdioHelper sdio_helper(&sdio);

// Read/write bytes
uint8_t val;
sdio_helper.ReadByte(0x1000, &val);
sdio_helper.WriteByte(0x1000, 0x42);

// Download firmware
sdio_helper.DownloadFirmware(fw_vmo, fw_size, 0x00100000);
```

### MMIO Helper (`mmio.h`)

Convenience methods for memory-mapped register access with bit manipulation.

**Usage:**
```cpp
#include "../../common/soliloquy_hal/mmio.h"

ddk::MmioBuffer mmio;
// ... initialize mmio ...
soliloquy_hal::MmioHelper mmio_helper(&mmio);

// Read/write registers
uint32_t val = mmio_helper.Read32(0x10);
mmio_helper.Write32(0x10, 0xDEADBEEF);

// Bit operations
mmio_helper.SetBits32(0x10, 0x0F);      // Set bits 0-3
mmio_helper.ClearBits32(0x10, 0xF0);    // Clear bits 4-7
mmio_helper.ModifyBits32(0x10, 0xFF, 0x42);  // Replace masked bits

// Wait for bit with timeout
bool ready = mmio_helper.WaitForBit32(0x10, 5, true, zx::msec(100));
```

### Clock/Reset Helper (`clock_reset.h`)

Manages clock gating and reset signals for peripherals.

**Usage:**
```cpp
#include "../../common/soliloquy_hal/clock_reset.h"

ddk::MmioBuffer ccu_mmio;
// ... initialize CCU MMIO ...
soliloquy_hal::ClockResetHelper clk_rst(&ccu_mmio);

// Enable clock and deassert reset
clk_rst.EnableClock(10);
clk_rst.DeassertReset(10);

// Disable peripheral
clk_rst.AssertReset(10);
clk_rst.DisableClock(10);
```

## Building

The HAL is built as a static library and linked into drivers.

**GN:**
```gn
deps = [
  "//drivers/common/soliloquy_hal",
]
```

**Bazel:**
```python
deps = [
    "//drivers/common/soliloquy_hal",
]
```

## Examples

- **WiFi Driver**: `drivers/wifi/aic8800/` - Uses SDIO helper and firmware loader
- **GPIO Driver**: `drivers/gpio/soliloquy_gpio/` - Uses MMIO helper

## Design Principles

1. **Thin wrappers**: HAL methods add minimal overhead over direct DDK calls
2. **Error propagation**: Return `zx_status_t` to allow proper error handling
3. **Logging**: HAL methods log errors to aid debugging
4. **Non-owning**: HAL objects don't own resources, drivers maintain ownership
