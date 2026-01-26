#ifndef DRIVERS_COMMON_SOLILOQUY_HAL_FIRMWARE_H_
#define DRIVERS_COMMON_SOLILOQUY_HAL_FIRMWARE_H_

#include <lib/zx/vmo.h>
#include <zircon/types.h>

namespace soliloquy_hal {

class FirmwareLoader {
public:
  static zx_status_t LoadFirmware(zx_device_t *parent, const char *name,
                                  zx::vmo *out_vmo, size_t *out_size);

  static zx_status_t MapFirmware(const zx::vmo &vmo, size_t size,
                                 uint8_t **out_data);
};

} // namespace soliloquy_hal

#endif // DRIVERS_COMMON_SOLILOQUY_HAL_FIRMWARE_H_
