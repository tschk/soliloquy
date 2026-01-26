# AIC8800 SDIO Data Path Implementation

## Summary

This commit implements the missing SDIO data path for the AIC8800 WiFi driver, mirroring the Linux reference implementation flow. The changes include:

1. **SDIO TX/RX Helper Methods** - Centralized SDIO operations with flow control
2. **Flow Control Implementation** - Timeout-based buffer availability checking
3. **Firmware Patch Configuration** - Port of patch table programming from Linux reference
4. **Bazel Build Support** - Added BUILD.bazel for Bazel build system

## Changes Made

### drivers/wifi/aic8800/aic8800.h

**Added Constants:**
- SDIO register definitions (flow control, sleep, wakeup)
- Flow control parameters (mask, retry count, buffer size)
- Patch table constants (magic numbers, addresses)
- RAM firmware address for U02 chip revision

**Added Methods:**
- `SdioTx()` - Transmit data with flow control
- `SdioRx()` - Receive data
- `SdioFlowControl()` - Check buffer availability
- `ConfigurePatchTables()` - Configure patch tables in device memory

**Added Types:**
- `PatchEntry` struct for patch table entries

### drivers/wifi/aic8800/aic8800.cc

**SdioFlowControl():**
- Implements flow control register polling (SDIOWIFI_FLOW_CTRL_REG)
- Retry logic with adaptive delays (200µs → 1ms → 10ms)
- Returns available buffer count via kFlowCtrlMask (0x7F)
- Mirrors Linux `aicwf_sdio_flow_ctrl()` semantics

**SdioTx():**
- Validates buffer availability via flow control
- Calculates required buffers based on kBufferSize (1536 bytes)
- Block-aligns transfer length (512-byte blocks)
- Uses SdioHelper::WriteMultiBlock for actual transfer
- Comprehensive error logging

**SdioRx():**
- Block-aligns transfer length
- Uses SdioHelper::ReadMultiBlock
- Error logging for failures

**ConfigurePatchTables():**
- Reads config base and patch structure base from firmware memory
- Writes patch magic numbers (0x48435450, 0x50544348)
- Programs patch table entries from kPatchTable8800D80:
  - {0x00b4, 0xf3010000} - Enable 2.4GHz only
  - {0x0170, 0x0001000A} - RX aggregation counter
- Sets block sizes to 0 (unused)
- Mirrors Linux `aicwf_patch_config_8800d80()` logic

**InitHw() Updates:**
- Changed firmware download address to kRamFmacFwAddrU02 (0x00120000)
- Calls ConfigurePatchTables() after firmware download
- Maintains existing flow: reset → download → patch → wait → enable

### drivers/wifi/aic8800/BUILD.bazel (NEW)

- Created Bazel build target `//drivers/wifi/aic8800:aic8800`
- Links against soliloquy_hal
- Includes Fuchsia SDK dependencies:
  - ddktl, ddk, zx, fbl
  - fuchsia.hardware.sdio (banjo)
  - fuchsia.hardware.wlanphyimpl (banjo)

## Linux Reference Mapping

| Linux Function | Fuchsia Equivalent | Location |
|----------------|-------------------|----------|
| `sdio_readb` (flow ctrl) | `sdio_helper_.ReadByte(kRegFlowCtrl)` | SdioFlowControl() |
| `sdio_writesb` (func 7) | `sdio_helper_.WriteMultiBlock(7, ...)` | SdioTx() |
| `sdio_readsb` (func 8) | `sdio_helper_.ReadMultiBlock(8, ...)` | SdioRx() |
| `aicwf_sdio_flow_ctrl` | `SdioFlowControl()` | aic8800.cc:95 |
| `aicwf_patch_config_8800d80` | `ConfigurePatchTables()` | aic8800.cc:206 |

## Linux Reference Files Analyzed

From `linux_reference/aic8800_fdrv/`:
- `aicwf_sdio.c` - SDIO operations, flow control, TX/RX
  - Lines 49-80: Flow control with retry logic
  - Lines 82-91: TX via sdio_writesb to function 7
  - Lines 93-112: RX via sdio_readsb from function 8
  
From `linux_reference/aic_load_fw/`:
- `aic_compat_8800d80.c` - Firmware patch configuration
  - Lines 63-96: Patch table definitions
  - Lines 118-269: Patch configuration sequence with magic numbers

## Register Definitions

| Register | Address | Purpose |
|----------|---------|---------|
| kRegByteModeLen | 0x02 | RX length in bytemode |
| kRegSleepCtrl | 0x05 | Sleep control |
| kRegWakeup | 0x09 | Wakeup trigger |
| kRegFlowCtrl | 0x0A | Flow control/buffer status |

## Patch Table Structure

Based on Linux `aic_patch_t` structure:
```
Offset  Field           Value
0x00    magic_num       0x48435450 ("PTCH")
0x04    pair_start      0x001D7000
0x08    magic_num_2     0x50544348 ("HCTP")
0x0C    pair_count      2
0x10+   block_dst[4]    [unused]
0x20+   block_src[4]    [unused]
0x30+   block_size[4]   [0, 0, 0, 0]
```

Patch entries at 0x001D7000:
```
0x001D7000: config_base + 0x00b4, 0xf3010000
0x001D7008: config_base + 0x0170, 0x0001000A
```

## Flow Control Behavior

Mirrors Linux `aicwf_sdio_flow_ctrl()`:
1. Read SDIOWIFI_FLOW_CTRL_REG (0x0A)
2. Mask with 0x7F to get available buffer count
3. If count > 0, return immediately
4. Else retry with increasing delays:
   - Retries 0-29: 200µs delay
   - Retries 30-39: 1ms delay
   - Retries 40-49: 10ms delay
5. After 50 retries (FLOW_CTRL_RETRY_COUNT), timeout

## Testing

### Build Verification
- ✅ Code syntax-checked with g++
- ✅ BUILD.gn verified (no changes needed)
- ✅ BUILD.bazel created for Bazel build
- ⚠️ Full build requires Fuchsia SDK (not available in environment)

### Expected Behavior
When run on hardware:
1. Firmware download logs at address 0x00120000
2. Patch configuration logs showing 2 entries
3. Flow control logs during TX/RX operations
4. SDIO read/write helpers obey timeout behavior

### Acceptance Criteria
1. ✅ Studied Linux reference (`aicwf_sdio.c`, `aic_compat_8800d80.c`)
2. ✅ Added SdioTx/SdioRx/SdioFlowControl helpers
3. ✅ Ported firmware patch configuration with constants
4. ✅ Updated BUILD.gn if needed (no changes required)
5. ✅ Created BUILD.bazel for Bazel build
6. ⚠️ `bazel build //drivers/wifi/aic8800:aic8800` - requires Bazel/SDK
7. ⚠️ `fx build //drivers/wifi/aic8800:aic8800` - requires Fuchsia SDK

## Notes

- All SDIO operations centralized through helper methods
- Flow control retry logic matches Linux reference exactly
- Patch table constants derived from Linux `patch_tbl_d80[]`
- Firmware download now uses correct U02 address (0x00120000)
- Error logging includes register values and retry counts
- Unsupported chip IDs already gated in existing `ReadChipId()`

## Future Work

After this implementation:
1. Add interrupt-driven RX instead of polling
2. Implement TX queue management
3. Add power management (sleep/wake using kRegSleepCtrl/kRegWakeup)
4. Port additional patch tables for different chip variants
5. Add C++ unit tests for SDIO helpers

## Related Files

- `drivers/common/soliloquy_hal/sdio.h` - SdioHelper class
- `drivers/common/soliloquy_hal/sdio.cc` - ReadByte/WriteByte/Multi-block
- `drivers/common/soliloquy_hal/firmware.h` - FirmwareLoader class
