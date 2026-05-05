# Driver Porting Guide

This guide walks through porting a Linux device driver to Soliloquy. We'll cover the key differences between Linux and Zircon driver models, how to leverage the Soliloquy HAL, and common pitfalls to avoid.

## Overview

Soliloquy uses the **Zircon Driver Development Kit (DDK)** instead of Linux's kernel driver API. While the core hardware interaction concepts remain the same, the APIs, initialization patterns, and lifecycle management differ significantly.

### Key Differences: Linux vs Zircon

| Aspect | Linux | Zircon/Soliloquy |
|--------|-------|------------------|
| **Driver Model** | Monolithic kernel modules | Userspace drivers communicating via IPC |
| **Bus Protocols** | `struct platform_device`, `struct sdio_func` | Banjo protocols (`ddk::SdioProtocolClient`) |
| **MMIO Access** | `ioread32()`, `iowrite32()` | `ddk::MmioBuffer` + `soliloquy_hal::MmioHelper` |
| **Interrupts** | `request_irq()`, ISR callbacks | `zx::interrupt`, async handlers |
| **Firmware Loading** | `request_firmware()` | `soliloquy_hal::FirmwareLoader` |
| **Locking** | `spinlock_t`, `mutex` | `fbl::Mutex`, Zircon futexes |
| **Memory Allocation** | `kmalloc()`, `dma_alloc_coherent()` | `zx::vmo` (Virtual Memory Objects) |
| **DMA** | `dma_map_single()` | `zx::bti` + pin/unpin operations |

## The Soliloquy HAL

The **Soliloquy Hardware Abstraction Layer** (`drivers/common/soliloquy_hal`) provides simplified wrappers around Zircon DDK primitives. All new drivers should use these helpers to reduce boilerplate and maintain consistency.

### HAL Components

#### 1. MMIO Access (`mmio.h`)

**Linux Equivalent:** `ioread32()`, `iowrite32()`, `readl()`, `writel()`

**Soliloquy HAL:**
```cpp
#include "../../common/soliloquy_hal/mmio.h"

soliloquy_hal::MmioHelper mmio_helper(mmio_buffer);

// Basic read/write (32-bit registers)
uint32_t value = mmio_helper.Read32(REGISTER_OFFSET);
mmio_helper.Write32(REGISTER_OFFSET, 0x1234);

// Bit manipulation
mmio_helper.SetBits32(CTRL_REG, ENABLE_BIT | START_BIT);
mmio_helper.ClearBits32(STATUS_REG, ERROR_FLAG);

// Modify specific bit fields
mmio_helper.ModifyBits32(CONFIG_REG, MODE_MASK, MODE_VALUE);

// Extract/write bit fields
uint32_t mode = mmio_helper.ReadMasked32(CONFIG_REG, MODE_MASK, MODE_SHIFT);
mmio_helper.WriteMasked32(CONFIG_REG, MODE_MASK, MODE_SHIFT, new_mode);

// Poll for hardware status
bool ready = mmio_helper.WaitForBit32(STATUS_REG, READY_BIT, true, zx::sec(1));
```

**Hardware Assumptions:**
- All registers are 32-bit aligned
- Register reads are idempotent (no side effects)
- Polling granularity: 10µs (suitable for most hardware)

#### 2. SDIO Helpers (`sdio.h`)

**Linux Equivalent:** `sdio_readb()`, `sdio_writeb()`, `sdio_memcpy_fromio()`

**Soliloquy HAL:**
```cpp
#include "../../common/soliloquy_hal/sdio.h"

ddk::SdioProtocolClient sdio;
// ... obtain sdio protocol from parent bus ...

soliloquy_hal::SdioHelper sdio_helper(&sdio);

// Byte-level access
uint8_t value;
zx_status_t status = sdio_helper.ReadByte(address, &value);
status = sdio_helper.WriteByte(address, 0xAB);

// Block transfers
uint8_t buffer[512];
status = sdio_helper.ReadMultiBlock(address, buffer, sizeof(buffer));
status = sdio_helper.WriteMultiBlock(address, data, length);

// Firmware download
zx::vmo fw_vmo = /* loaded firmware VMO */;
status = sdio_helper.DownloadFirmware(fw_vmo, fw_size, BASE_ADDRESS);
```

**Key Differences from Linux:**
- Operations return `zx_status_t` instead of negative error codes
- Block size is fixed at 512 bytes (configurable in HAL if needed)
- Firmware download is a single HAL call instead of manual chunking

#### 3. Firmware Loading (`firmware.h`)

**Linux Equivalent:** `request_firmware()`, `release_firmware()`

**Soliloquy HAL:**
```cpp
#include "../../common/soliloquy_hal/firmware.h"

// Load firmware into a VMO
zx::vmo fw_vmo;
size_t fw_size;
zx_status_t status = soliloquy_hal::FirmwareLoader::LoadFirmware(
    parent, "aic8800/fw.bin", &fw_vmo, &fw_size);

if (status == ZX_OK) {
  // Map the VMO to read firmware data
  zx_vaddr_t fw_data;
  zx::vmar::root_self()->map(ZX_VM_PERM_READ, 0, fw_vmo, 0, fw_size, &fw_data);
  
  // Process firmware...
  
  zx::vmar::root_self()->unmap(fw_data, fw_size);
}
```

**Firmware Location:**
- Place firmware files in `drivers/<category>/<driver>/firmware/`
- Reference them by path in `load_firmware_from_driver()`
- Firmware is packaged into the driver's component manifest

#### 4. Clock/Reset Control (`clock_reset.h`)

**Linux Equivalent:** `clk_prepare_enable()`, `reset_control_deassert()`

**Soliloquy HAL:**
```cpp
#include "../../common/soliloquy_hal/clock_reset.h"

// Enable clocks and deassert resets
soliloquy_hal::ClockResetHelper clock_reset(clock_protocol, reset_protocol);

status = clock_reset.EnableClock(CLOCK_ID);
status = clock_reset.ResetDeassert(RESET_ID);

// Cleanup
status = clock_reset.ResetAssert(RESET_ID);
status = clock_reset.DisableClock(CLOCK_ID);
```

## Porting Workflow

### Step 1: Analyze the Linux Driver

Identify key components:
1. **Bus interface**: Platform device? SDIO? PCI? USB?
2. **MMIO regions**: Register map and offsets
3. **Interrupts**: IRQ numbers and handler logic
4. **Firmware**: Required files and load sequence
5. **DMA**: Coherent buffers, streaming DMA
6. **Power management**: Clock enables, resets

**Example: AIC8800 WiFi Driver**
```c
// Linux SDIO registration
static struct sdio_driver aic8800_sdio_driver = {
    .probe = aic8800_probe,
    .remove = aic8800_remove,
    .id_table = aic8800_ids,
};
module_sdio_driver(aic8800_sdio_driver);
```

### Step 2: Create Driver Structure

Create directory and initial files:
```bash
mkdir -p drivers/<category>/<driver>
cd drivers/<category>/<driver>

# Create source files
touch <driver>.cc <driver>.h

# Create BUILD.gn
cat > BUILD.gn << 'EOF'
import("//build/bind/bind.gni")
import("//build/drivers.gni")

driver_bind_rules("bind") {
  rules = "<driver>.bind"
  header_output = "<driver>-bind.h"
}

fuchsia_driver("<driver>") {
  sources = [ "<driver>.cc" ]
  deps = [
    ":bind",
    "//drivers/common/soliloquy_hal",
    "//src/devices/lib/driver",
    "//src/lib/ddk",
  ]
}

fuchsia_driver_component("<driver>-component") {
  component_name = "<driver>"
  deps = [ ":<driver>" ]
  info = "<driver>-info.json"
}
EOF
```

### Step 3: Implement Driver Class

Use Zircon DDK device lifecycle:
```cpp
#include <ddktl/device.h>
#include "../../common/soliloquy_hal/mmio.h"
#include "../../common/soliloquy_hal/sdio.h"
#include "../../common/soliloquy_hal/firmware.h"

class MyDriver : public ddk::Device<MyDriver, ddk::Initializable, ddk::Unbindable> {
 public:
  MyDriver(zx_device_t* parent, ddk::SdioProtocolClient sdio, ddk::MmioBuffer mmio)
      : ddk::Device<MyDriver, ddk::Initializable, ddk::Unbindable>(parent),
        sdio_helper_(&sdio),
        mmio_helper_(&mmio) {}
  
  static zx_status_t Bind(void* ctx, zx_device_t* parent);
  
  // DDK lifecycle hooks
  void DdkInit(ddk::InitTxn txn);
  void DdkUnbind(ddk::UnbindTxn txn);
  void DdkRelease();

 private:
  zx_status_t InitHardware();
  zx_status_t LoadFirmware();
  
  soliloquy_hal::SdioHelper sdio_helper_;
  soliloquy_hal::MmioHelper mmio_helper_;
};
```

### Step 4: Map Linux APIs to Soliloquy HAL

#### MMIO Register Access

**Linux:**
```c
void __iomem *base = ioremap(phys_addr, size);
u32 val = readl(base + OFFSET);
writel(val | BIT(3), base + OFFSET);
iounmap(base);
```

**Soliloquy:**
```cpp
// MMIO buffer obtained in Bind() from parent device
uint32_t val = mmio_helper_.Read32(OFFSET);
mmio_helper_.SetBits32(OFFSET, BIT(3));
// No explicit unmap needed - handled by MmioBuffer destructor
```

#### SDIO Operations

**Linux:**
```c
sdio_claim_host(func);
u8 val = sdio_readb(func, addr, &ret);
sdio_writeb(func, val, addr, &ret);
sdio_release_host(func);
```

**Soliloquy:**
```cpp
uint8_t val;
zx_status_t status = sdio_helper_.ReadByte(addr, &val);
status = sdio_helper_.WriteByte(addr, val);
// No explicit locking - SDIO protocol client handles synchronization
```

#### Firmware Loading

**Linux:**
```c
const struct firmware *fw;
int ret = request_firmware(&fw, "device/fw.bin", dev);
if (ret == 0) {
  // Use fw->data and fw->size
  release_firmware(fw);
}
```

**Soliloquy:**
```cpp
zx::vmo fw_vmo;
size_t fw_size;
zx_status_t status = soliloquy_hal::FirmwareLoader::LoadFirmware(
    parent, "device/fw.bin", &fw_vmo, &fw_size);
if (status == ZX_OK) {
  // Use fw_vmo (VMO automatically cleaned up by RAII)
}
```

### Step 5: Implement Device Binding

Create bind rules to match hardware:
```cpp
// <driver>.bind
using fuchsia.platform;
using fuchsia.sdio;

fuchsia.BIND_PROTOCOL == fuchsia.sdio.BIND_PROTOCOL.DEVICE;
fuchsia.BIND_SDIO_VID == 0x3030;  // Vendor ID
fuchsia.BIND_SDIO_PID == 0x8800;  // Product ID
```

Implement `Bind()` function:
```cpp
zx_status_t MyDriver::Bind(void* ctx, zx_device_t* parent) {
  // Get SDIO protocol from parent
  ddk::SdioProtocolClient sdio(parent);
  if (!sdio.is_valid()) {
    return ZX_ERR_NOT_SUPPORTED;
  }
  
  // Get MMIO resource
  ddk::MmioBuffer mmio;
  zx_status_t status = ddk::MmioBuffer::Create(/* resource */, &mmio);
  if (status != ZX_OK) {
    return status;
  }
  
  // Create driver instance
  auto driver = std::make_unique<MyDriver>(parent, std::move(sdio), std::move(mmio));
  
  // Add device to tree
  status = driver->DdkAdd("my-driver");
  if (status == ZX_OK) {
    driver.release();  // Ownership transferred to DDK
  }
  return status;
}
```

### Step 6: Register in Board Configuration

Add device to board initialization:
```cpp
// boards/arm64/soliloquy/src/soliloquy-<device>.cc

zx_status_t Soliloquy::MyDeviceInit() {
  pbus_dev_t dev = {};
  dev.name = "my-device";
  dev.vid = PDEV_VID_GENERIC;
  dev.pid = PDEV_PID_GENERIC;
  dev.did = PDEV_DID_MY_DEVICE;
  
  // Define MMIO resources
  static const pbus_mmio_t mmio[] = {
      {.base = MMIO_BASE_ADDRESS, .length = MMIO_SIZE},
  };
  dev.mmio_list = mmio;
  dev.mmio_count = std::size(mmio);
  
  // Define interrupts (if needed)
  static const pbus_irq_t irqs[] = {
      {.irq = IRQ_NUMBER, .mode = ZX_INTERRUPT_MODE_EDGE_HIGH},
  };
  dev.irq_list = irqs;
  dev.irq_count = std::size(irqs);
  
  return pbus_.DeviceAdd(&dev);
}
```

Add initialization to board startup:
```cpp
// boards/arm64/soliloquy/src/soliloquy.cc

zx_status_t Soliloquy::Start() {
  // ... existing init ...
  
  if (status = MyDeviceInit(); status != ZX_OK) {
    zxlogf(ERROR, "MyDeviceInit failed: %s", zx_status_get_string(status));
  }
  
  return ZX_OK;
}
```

## Common Pitfalls

### 1. Error Code Mapping

**Linux:** Negative errno values (`-ENOMEM`, `-EINVAL`)  
**Zircon:** `zx_status_t` enum values (`ZX_ERR_NO_MEMORY`, `ZX_ERR_INVALID_ARGS`)

Always use `ZX_ERR_*` constants, not negative numbers.

### 2. Memory Management

**Linux:** `kmalloc()`/`kfree()`  
**Zircon:** RAII with `std::unique_ptr`, `fbl::RefPtr`, or `zx::vmo`

Never use raw `new`/`delete` for driver objects managed by DDK.

### 3. Threading and Locks

**Linux:** `spin_lock_irqsave()`, `mutex_lock()`  
**Zircon:** `fbl::Mutex`, async dispatcher loops

Zircon drivers prefer async event handling over blocking locks. Use `async::Loop` for deferred work.

### 4. Firmware Paths

**Linux:** Searches `/lib/firmware/`  
**Zircon:** Embedded in driver package

Place firmware in driver directory and reference by relative path. Update `BUILD.gn` to include files:
```gn
resource("firmware") {
  sources = [ "firmware/fw.bin" ]
  outputs = [ "data/{{source_file_part}}" ]
}
```

### 5. DMA Buffers

**Linux:** `dma_alloc_coherent()` returns virtual + physical addresses  
**Zircon:** `zx::vmo` + `zx::bti::pin()` for DMA

Must explicitly pin VMOs to get physical addresses for DMA:
```cpp
zx::bti bti = /* obtain from parent */;
zx::vmo dma_vmo;
zx::vmo::create(size, 0, &dma_vmo);
zx::pmt pmt;
zx_paddr_t phys_addr;
bti.pin(ZX_BTI_PERM_READ | ZX_BTI_PERM_WRITE, dma_vmo, 0, size, &phys_addr, 1, &pmt);
```

### 6. Interrupt Handling

**Linux:** `request_irq()` + ISR callback  
**Zircon:** `zx::interrupt` + async wait

```cpp
zx::interrupt irq;
zx_status_t status = zx::interrupt::create(resource, irq_num, ZX_INTERRUPT_REMAP_IRQ, &irq);

// Wait for interrupt asynchronously
async::Wait wait(irq.get(), ZX_INTERRUPT_SIGNALED);
wait.Begin(dispatcher, [this](async_dispatcher_t*, async::Wait*, zx_status_t, const zx_packet_signal_t*) {
  irq_.ack();
  HandleInterrupt();
});
```

## Example: AIC8800 WiFi Driver Port

The AIC8800 driver demonstrates a complete Linux→Soliloquy port:

**Location:** `drivers/wifi/aic8800/`

**Key changes from Linux version:**
1. SDIO operations use `soliloquy_hal::SdioHelper` instead of `sdio_claim_host()` patterns
2. Firmware loading simplified to single `DownloadFirmware()` call
3. Register access via `MmioHelper` for consistency
4. DDK lifecycle hooks replace Linux module init/exit

**Firmware Download Example:**
```cpp
zx_status_t Aic8800::LoadAndDownloadFirmware() {
  zx::vmo fw_vmo;
  size_t fw_size;
  
  zx_status_t status = soliloquy_hal::FirmwareLoader::LoadFirmware(
      parent(), "aic8800/fmacfw.bin", &fw_vmo, &fw_size);
  if (status != ZX_OK) {
    return status;
  }
  
  return sdio_helper_.DownloadFirmware(fw_vmo, fw_size, FIRMWARE_BASE_ADDR);
}
```

## Testing and Debugging

### 1. Enable Driver Logging

```cpp
#include <lib/ddk/debug.h>

zxlogf(INFO, "Driver initialized successfully");
zxlogf(ERROR, "Failed to load firmware: %s", zx_status_get_string(status));
```

View logs:
```bash
fx log --tag my-driver
```

### 2. Unit Tests

Create unit tests using Zircon's fake DDK:
```cpp
#include <lib/fake_ddk/fake_ddk.h>

TEST(MyDriverTest, BasicInit) {
  fake_ddk::Bind bind;
  
  zx_device_t* parent = fake_ddk::kFakeParent;
  ASSERT_OK(MyDriver::Bind(nullptr, parent));
}
```

### 3. Hardware Testing

Flash driver to device:
```bash
fx set minimal.arm64 --board soliloquy
fx build
./tools/soliloquy/debug.sh
```

Monitor serial console:
```bash
./tools/soliloquy/debug.sh
```

## Checklist

Before submitting your ported driver:

- [ ] Uses `soliloquy_hal` helpers for MMIO/SDIO/firmware
- [ ] Proper DDK lifecycle implementation (Bind/Init/Unbind/Release)
- [ ] Error codes use `ZX_ERR_*` instead of negative errno
- [ ] No raw `new`/`delete` - uses RAII/smart pointers
- [ ] Firmware files included in `BUILD.gn` resources
- [ ] Device added to board initialization
- [ ] Bind rules defined for hardware matching
- [ ] Logging uses `zxlogf()` with appropriate levels
- [ ] Builds without warnings: `fx build drivers/<category>/<driver>`
- [ ] Tested on target hardware or emulator

## Additional Resources

- **Zircon DDK Documentation**: [fuchsia.dev/fuchsia-src/development/drivers](https://fuchsia.dev/fuchsia-src/development/drivers)
- **Soliloquy HAL Source**: `drivers/common/soliloquy_hal/`
- **Reference Drivers**:
  - GPIO: `drivers/gpio/soliloquy_gpio/`
  - WiFi (SDIO): `drivers/wifi/aic8800/`
- **Board Configuration**: `boards/arm64/soliloquy/`

## Getting Help

If you encounter issues during porting:

1. Check existing drivers for similar hardware patterns
2. Review Zircon DDK examples in Fuchsia source tree
3. Verify HAL usage matches documented patterns
4. Enable verbose logging (`zxlogf(DEBUG, ...)`)
5. Test incrementally - get basic bind working before adding complexity

For questions specific to Soliloquy's architecture or HAL design, refer to the [Developer Guide](../DEVELOPER_GUIDE.md).
