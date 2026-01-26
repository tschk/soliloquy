#include "firmware.h"

#include <lib/ddk/debug.h>
#include <lib/ddk/device.h>
#include <zircon/status.h>

namespace soliloquy_hal {

zx_status_t FirmwareLoader::LoadFirmware(zx_device_t *parent, const char *name,
                                         zx::vmo *out_vmo, size_t *out_size) {
  if (!parent || !name || !out_vmo || !out_size) {
    return ZX_ERR_INVALID_ARGS;
  }

  zx_status_t status =
      load_firmware(parent, name, out_vmo->reset_and_get_address(), out_size);
  if (status != ZX_OK) {
    zxlogf(ERROR, "soliloquy_hal: Failed to load firmware '%s': %s", name,
           zx_status_get_string(status));
    return status;
  }

  zxlogf(INFO, "soliloquy_hal: Loaded firmware '%s' (%zu bytes)", name,
         *out_size);
  return ZX_OK;
}

zx_status_t FirmwareLoader::MapFirmware(const zx::vmo &vmo, size_t size,
                                        uint8_t **out_data) {
  if (!out_data || size == 0) {
    return ZX_ERR_INVALID_ARGS;
  }

  zx_vaddr_t mapped_addr;
  zx_status_t status = zx_vmar_map(zx_vmar_root_self(), ZX_VM_PERM_READ, 0,
                                   vmo.get(), 0, size, &mapped_addr);
  if (status != ZX_OK) {
    zxlogf(ERROR, "soliloquy_hal: Failed to map firmware VMO: %s",
           zx_status_get_string(status));
    return status;
  }

  *out_data = reinterpret_cast<uint8_t *>(mapped_addr);
  return ZX_OK;
}

} // namespace soliloquy_hal
