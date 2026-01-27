#include "sdio.h"

#include <lib/ddk/debug.h>
#include <zircon/status.h>

namespace soliloquy_hal {

zx_status_t SdioHelper::ReadByte(uint32_t addr, uint8_t *out_val) {
  if (!out_val) {
    return ZX_ERR_INVALID_ARGS;
  }
  return sdio_->DoRwByte(false, addr, 0, out_val);
}

zx_status_t SdioHelper::WriteByte(uint32_t addr, uint8_t val) {
  return sdio_->DoRwByte(true, addr, val, nullptr);
}

zx_status_t SdioHelper::WriteMultiBlock(uint8_t func, const uint8_t* buf, size_t len) {
  // TODO: Implement proper multi-block write using DoRwTxn
  // This requires VMO setup and mapping which is non-trivial for a simple helper.
  // For now, this is a stub to allow drivers to compile.
  
  // To be partially functional, we could loop DoRwByte if we knew the address.
  // But this API doesn't take an address?
  return ZX_OK;
}

zx_status_t SdioHelper::ReadMultiBlock(uint8_t func, uint8_t* buf, size_t len) {
  // TODO: Implement proper multi-block read
  return ZX_OK;
}

zx_status_t SdioHelper::DownloadFirmware(const zx::vmo& vmo, size_t size, uint32_t addr) {
  // TODO: Implement firmware download
  return ZX_OK;
}

} // namespace soliloquy_hal
