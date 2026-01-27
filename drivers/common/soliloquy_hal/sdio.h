#ifndef DRIVERS_COMMON_SOLILOQUY_HAL_SDIO_H_
#define DRIVERS_COMMON_SOLILOQUY_HAL_SDIO_H_

#include <fuchsia/hardware/sdio/cpp/banjo.h>
#include <lib/zx/vmo.h>
#include <zircon/types.h>

namespace soliloquy_hal {

// Simple SDIO helper wrapper around the banjo SDIO protocol client.
// This provides basic byte-level and buffer I/O operations.
class SdioHelper {
public:
  explicit SdioHelper(ddk::SdioProtocolClient *sdio) : sdio_(sdio) {}

  // Read a single byte from an SDIO address
  zx_status_t ReadByte(uint32_t addr, uint8_t *out_val);
  
  // Write a single byte to an SDIO address
  zx_status_t WriteByte(uint32_t addr, uint8_t val);

  // Write multiple blocks/bytes to a function
  zx_status_t WriteMultiBlock(uint8_t func, const uint8_t* buf, size_t len);

  // Read multiple blocks/bytes from a function
  zx_status_t ReadMultiBlock(uint8_t func, uint8_t* buf, size_t len);

  // Download firmware from VMO to device
  zx_status_t DownloadFirmware(const zx::vmo& vmo, size_t size, uint32_t addr);

private:
  ddk::SdioProtocolClient *sdio_;
};

} // namespace soliloquy_hal

#endif // DRIVERS_COMMON_SOLILOQUY_HAL_SDIO_H_
