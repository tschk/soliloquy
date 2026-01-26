# AIC8800 Linux Reference Mapping

This document describes how the Linux AIC8800 SDIO driver maps to the Fuchsia implementation.

## Overview

The AIC8800 WiFi chipset uses SDIO for communication and requires firmware to be loaded at initialization. The Linux driver located in `vendor/aic8800-linux` provides the reference implementation.

## Key Linux Source Files

- **aicwf_sdio.c**: Main SDIO interface implementation
- **aicwf_sdio.h**: SDIO register definitions and structures
- **aic_bsp_driver.c**: Board support and initialization
- **rwnx_main.c**: mac80211 integration
- **README**: Chip variants and firmware requirements

## Register Mapping

### Linux → Fuchsia Register Constants

| Linux (aicwf_sdio.h) | Fuchsia (aic8800.h) | Description |
|----------------------|---------------------|-------------|
| `SDIOWIFI_FUNC_BLOCKSIZE` | `Aic8800Registers::BLOCK_SIZE_DEFAULT` | Default SDIO block size (512 bytes) |
| `HOST_CTRL_REG` | `REG_HOST_CTRL` | Host control register |
| `HOST_INT_STATUS_REG` | `REG_INT_STATUS` | Interrupt status register |
| `FW_DOWNLOAD_ADDR` | `REG_FW_DOWNLOAD_ADDR` | Firmware download base address |
| `CHIPID_ADDR` | `REG_CHIP_ID` | Chip ID register |

### Control Bits

| Linux Constant | Fuchsia Constant | Purpose |
|----------------|------------------|---------|
| `HOST_CTRL_RESET_BIT` | `HOST_CTRL_RESET` | Reset the chip |
| `HOST_CTRL_EN_BIT` | `HOST_CTRL_ENABLE` | Enable the chip |
| `INT_FW_READY_BIT` | `INT_FW_READY` | Firmware ready interrupt |
| `INT_ERROR_BIT` | `INT_ERROR` | Error interrupt flag |

## SDIO Operations Mapping

### Byte Operations

**Linux:**
```c
u8 aicwf_sdio_readb(struct aic_sdio_dev *sdiodev, u32 addr) {
    u8 data;
    sdio_claim_host(sdiodev->func);
    data = sdio_readb(sdiodev->func, addr, &ret);
    sdio_release_host(sdiodev->func);
    return data;
}
```

**Fuchsia (C++):**
```cpp
zx_status_t SdioHelper::ReadByte(uint32_t addr, uint8_t *out_val) {
    return sdio_->DoRwByte(false, addr, 0, out_val);
}
```

**Fuchsia (Rust Mock):**
```rust
pub fn read_byte(&self, address: u32) -> SdioResult<u8> {
    let memory = self.memory.lock().unwrap();
    let value = memory.get(&address).copied().unwrap_or(0);
    Ok(value)
}
```

### Block Operations

**Linux:**
```c
int aicwf_sdio_send_pkt(struct aic_sdio_dev *sdiodev, u8 *buf, u32 count) {
    sdio_claim_host(func);
    ret = sdio_memcpy_toio(func, SDIOWIFI_FUNC_BLOCKSIZE, buf, count);
    sdio_release_host(func);
    return ret;
}
```

**Fuchsia (C++):**
```cpp
zx_status_t SdioHelper::WriteMultiBlock(uint32_t addr, const uint8_t *buf, size_t len) {
    size_t blocks = (len + kBlockSize - 1) / kBlockSize;
    for (size_t i = 0; i < blocks; i++) {
        status = sdio_->DoRwTxn(addr + (i * kBlockSize), buf + (i * kBlockSize), 
                                chunk_size, true, false);
    }
    return status;
}
```

**Fuchsia (Rust Mock):**
```rust
pub fn write_multi_block(&self, address: u32, data: &[u8]) -> SdioResult<()> {
    let mut memory = self.memory.lock().unwrap();
    for (i, &byte) in data.iter().enumerate() {
        let addr = address.wrapping_add(i as u32);
        memory.insert(addr, byte);
    }
    Ok(())
}
```

## Firmware Loading

### Linux Firmware Download Sequence

```c
static int aicwf_sdio_download_fw(struct aic_sdio_dev *sdiodev) {
    // 1. Request firmware
    ret = request_firmware(&fw, fw_name, sdiodev->dev);
    
    // 2. Write firmware to device RAM
    addr = FW_DOWNLOAD_ADDR;
    for (offset = 0; offset < fw->size; offset += block_size) {
        aicwf_sdio_send_pkt(sdiodev, fw->data + offset, block_size);
        addr += block_size;
    }
    
    // 3. Set firmware status
    aicwf_sdio_writeb(sdiodev, FW_DOWNLOAD_ADDR_REG, FW_READY_FLAG);
    
    return 0;
}
```

### Fuchsia Firmware Download Sequence

**C++ Driver:**
```cpp
zx_status_t Aic8800::InitHw() {
    // 1. Load firmware from package
    zx::vmo fw_vmo;
    size_t fw_size;
    status = soliloquy_hal::FirmwareLoader::LoadFirmware(
        parent(), kFwName, &fw_vmo, &fw_size);
    
    // 2. Download via SDIO
    status = sdio_helper_.DownloadFirmware(fw_vmo, fw_size, kFirmwareBaseAddr);
    
    return status;
}
```

**Rust Mock:**
```rust
pub fn download_firmware(&self, base_address: u32, firmware_data: &[u8]) -> SdioResult<()> {
    let block_size = self.get_block_size();
    let blocks = (firmware_data.len() + block_size - 1) / block_size;
    
    for block_idx in 0..blocks {
        let start = block_idx * block_size;
        let end = std::cmp::min(start + block_size, firmware_data.len());
        let block_data = &firmware_data[start..end];
        let block_addr = base_address + (block_idx * block_size) as u32;
        
        self.write_multi_block(block_addr, block_data)?;
    }
    
    Ok(())
}
```

## Initialization Sequence

### Linux Driver Probe Sequence

```c
static int aicwf_sdio_probe(struct sdio_func *func, const struct sdio_device_id *id) {
    // 1. Enable function
    sdio_enable_func(func);
    
    // 2. Set block size
    sdio_set_block_size(func, SDIOWIFI_FUNC_BLOCKSIZE);
    
    // 3. Read chip ID
    chip_id = aicwf_sdio_readl(sdiodev, CHIPID_ADDR);
    
    // 4. Download firmware
    aicwf_sdio_download_fw(sdiodev);
    
    // 5. Wait for firmware ready
    wait_for_fw_ready(sdiodev);
    
    // 6. Register network device
    register_netdev(ndev);
    
    return 0;
}
```

### Fuchsia Driver Init Sequence

```cpp
zx_status_t Aic8800::InitHw() {
    // 1. Initialize SDIO
    // (Handled by parent device)
    
    // 2. Set block size
    // (Handled in SdioHelper constructor)
    
    // 3. Read chip ID
    uint8_t chip_id;
    sdio_helper_.ReadByte(kRegChipId, &chip_id);
    
    // 4. Load and download firmware
    zx::vmo fw_vmo;
    size_t fw_size;
    status = FirmwareLoader::LoadFirmware(parent(), kFwName, &fw_vmo, &fw_size);
    status = sdio_helper_.DownloadFirmware(fw_vmo, fw_size, kFirmwareBaseAddr);
    
    // 5. Wait for firmware ready
    // TODO: Poll REG_FW_STATUS
    
    // 6. Add WLANPHY device
    // (Handled by DdkAdd)
    
    return ZX_OK;
}
```

## Chip Variants

### Supported Chips (from Linux README)

| Chip ID | Variant | Firmware Files |
|---------|---------|----------------|
| 0x88000000 | AIC8800D | fmacfw_8800d80.bin |
| 0x88000001 | AIC8800DC | fmacfw_8800dc.bin, fmacfw_patch_8800dc_u02.bin |
| 0x88000002 | AIC8800DW | fmacfw_8800dw.bin |

### Firmware File Format

The firmware binary has the following structure (from Linux aicwf_sdio.c):

```
Offset | Size | Description
-------|------|------------
0x00   | 8    | Magic "AIC8800\x01"
0x08   | 4    | Header size (little-endian)
0x0C   | 4    | Code size (little-endian)
0x10   | 4    | Data offset (little-endian)
...    | ...  | Padding to header size
header | code | ARM code section
       | size | 
```

## Interrupt Handling

### Linux IRQ Handler

```c
static irqreturn_t aicwf_sdio_irq_handler(int irq, void *dev_id) {
    u32 intstatus;
    
    // Read interrupt status
    intstatus = aicwf_sdio_readl(sdiodev, HOST_INT_STATUS_REG);
    
    // Handle specific interrupts
    if (intstatus & INT_FW_READY_BIT) {
        // Firmware is ready
        complete(&sdiodev->fw_ready);
    }
    
    if (intstatus & INT_RX_READY_BIT) {
        // RX data available
        aicwf_sdio_rx_handler(sdiodev);
    }
    
    if (intstatus & INT_TX_DONE_BIT) {
        // TX complete
        aicwf_sdio_tx_complete(sdiodev);
    }
    
    // Clear interrupts
    aicwf_sdio_writel(sdiodev, HOST_INT_STATUS_REG, intstatus);
    
    return IRQ_HANDLED;
}
```

### Fuchsia Interrupt Handling (TODO)

The Fuchsia driver needs to implement interrupt handling through the SDIO protocol:

```cpp
// TODO: Implement interrupt handler
void Aic8800::HandleInterrupt() {
    uint32_t int_status;
    
    // Read interrupt status (4 bytes)
    uint8_t status_bytes[4];
    for (int i = 0; i < 4; i++) {
        sdio_helper_.ReadByte(kRegIntStatus + i, &status_bytes[i]);
    }
    int_status = *reinterpret_cast<uint32_t*>(status_bytes);
    
    // Handle interrupts
    if (int_status & kIntFwReady) {
        // Firmware ready
    }
    
    if (int_status & kIntRxReady) {
        // RX data available
    }
    
    if (int_status & kIntTxDone) {
        // TX complete
    }
    
    // Clear interrupts
    for (int i = 0; i < 4; i++) {
        sdio_helper_.WriteByte(kRegIntStatus + i, status_bytes[i]);
    }
}
```

## Testing Strategy

### Mock Testing (Rust)

The mock utilities allow testing the driver logic without real hardware:

```rust
// Create mock device
let sdio = MockSdioDevice::new();
sdio.initialize().unwrap();
sdio.set_block_size(512).unwrap();

// Create mock firmware
let fw_loader = MockFirmwareLoader::new();
let firmware = MockFirmwareLoader::create_aic8800_firmware();
fw_loader.add_firmware("fmacfw_8800d80.bin", firmware);

// Test firmware download
let fw_data = fw_loader.load_firmware("fmacfw_8800d80.bin").unwrap();
sdio.download_firmware(0x00100000, &fw_data).unwrap();

// Verify firmware in memory
assert!(sdio.verify_firmware_at(0x00100000, &fw_data));
```

### Integration Testing

The integration tests verify the complete initialization flow:

1. SDIO device initialization
2. Block size configuration
3. Firmware loading
4. Firmware download via SDIO
5. Verification of downloaded data

## Future Work

1. **Interrupt Handling**: Implement proper interrupt handling for FW ready, RX/TX events
2. **WLANPHY Protocol**: Complete implementation of WlanphyImpl methods
3. **Interface Management**: Add support for creating/destroying WiFi interfaces
4. **Power Management**: Implement sleep/wake sequences from Linux driver
5. **Error Recovery**: Add robust error handling and recovery mechanisms
6. **Performance Optimization**: Optimize SDIO transfer sizes and DMA if available

## References

- Linux driver source: `vendor/aic8800-linux/`
- Fuchsia SDIO protocol: `//sdk/banjo/fuchsia.hardware.sdio`
- Fuchsia WLANPHY protocol: `//sdk/banjo/fuchsia.hardware.wlanphyimpl`
