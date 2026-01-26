#ifndef DRIVERS_COMMON_SOLILOQUY_HAL_SDIO_H_
#define DRIVERS_COMMON_SOLILOQUY_HAL_SDIO_H_

#include <ddktl/protocol/sdio.h>
#include <fuchsia/hardware/sdio/cpp/banjo.h>
#include <lib/zx/vmo.h>
#include <zircon/types.h>

namespace soliloquy_hal {

class SdioHelper {
public:
  explicit SdioHelper(ddk::SdioProtocolClient *sdio) : sdio_(sdio) {}

  zx_status_t ReadByte(uint32_t addr, uint8_t *out_val);
  zx_status_t WriteByte(uint32_t addr, uint8_t val);

  zx_status_t ReadMultiBlock(uint32_t addr, uint8_t *buf, size_t len);
  zx_status_t WriteMultiBlock(uint32_t addr, const uint8_t *buf, size_t len);

  zx_status_t DownloadFirmware(const zx::vmo &fw_vmo, size_t size,
                               uint32_t base_addr);

private:
  ddk::SdioProtocolClient *sdio_;
  static constexpr size_t kBlockSize = 512;
};

} // namespace soliloquy_hal

#endif // DRIVERS_COMMON_SOLILOQUY_HAL_SDIO_H_
