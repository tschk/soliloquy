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

zx_status_t SdioHelper::ReadMultiBlock(uint32_t addr, uint8_t *buf,
                                       size_t len) {
  if (!buf || len == 0) {
    return ZX_ERR_INVALID_ARGS;
  }

  size_t blocks = (len + kBlockSize - 1) / kBlockSize;
  zx_status_t status = ZX_OK;

  for (size_t i = 0; i < blocks; i++) {
    size_t chunk_size = (i == blocks - 1) ? (len % kBlockSize) : kBlockSize;
    if (chunk_size == 0)
      chunk_size = kBlockSize;

    status = sdio_->DoRwTxn(addr + (i * kBlockSize), buf + (i * kBlockSize),
                            chunk_size, false, false);
    if (status != ZX_OK) {
      zxlogf(ERROR, "soliloquy_hal: SDIO read block %zu failed: %s", i,
             zx_status_get_string(status));
      return status;
    }
  }

  return ZX_OK;
}

zx_status_t SdioHelper::WriteMultiBlock(uint32_t addr, const uint8_t *buf,
                                        size_t len) {
  if (!buf || len == 0) {
    return ZX_ERR_INVALID_ARGS;
  }

  size_t blocks = (len + kBlockSize - 1) / kBlockSize;
  zx_status_t status = ZX_OK;

  for (size_t i = 0; i < blocks; i++) {
    size_t chunk_size = (i == blocks - 1) ? (len % kBlockSize) : kBlockSize;
    if (chunk_size == 0)
      chunk_size = kBlockSize;

    status = sdio_->DoRwTxn(addr + (i * kBlockSize),
                            const_cast<uint8_t *>(buf + (i * kBlockSize)),
                            chunk_size, true, false);
    if (status != ZX_OK) {
      zxlogf(ERROR, "soliloquy_hal: SDIO write block %zu failed: %s", i,
             zx_status_get_string(status));
      return status;
    }
  }

  return ZX_OK;
}

zx_status_t SdioHelper::DownloadFirmware(const zx::vmo &fw_vmo, size_t size,
                                         uint32_t base_addr) {
  if (size == 0) {
    return ZX_ERR_INVALID_ARGS;
  }

  zxlogf(INFO,
         "soliloquy_hal: Downloading firmware via SDIO (%zu bytes to 0x%x)",
         size, base_addr);

  zx_vaddr_t mapped_addr;
  zx_status_t status = zx_vmar_map(zx_vmar_root_self(), ZX_VM_PERM_READ, 0,
                                   fw_vmo.get(), 0, size, &mapped_addr);
  if (status != ZX_OK) {
    zxlogf(ERROR, "soliloquy_hal: Failed to map firmware VMO: %s",
           zx_status_get_string(status));
    return status;
  }

  uint8_t *fw_data = reinterpret_cast<uint8_t *>(mapped_addr);
  status = WriteMultiBlock(base_addr, fw_data, size);

  zx_vmar_unmap(zx_vmar_root_self(), mapped_addr, size);

  if (status != ZX_OK) {
    zxlogf(ERROR, "soliloquy_hal: Firmware download failed: %s",
           zx_status_get_string(status));
    return status;
  }

  zxlogf(INFO, "soliloquy_hal: Firmware download complete");
  return ZX_OK;
}

} // namespace soliloquy_hal
