#ifndef DRIVERS_COMMON_SOLILOQUY_HAL_MMIO_H_
#define DRIVERS_COMMON_SOLILOQUY_HAL_MMIO_H_

#include <lib/mmio/mmio.h>
#include <zircon/types.h>

namespace soliloquy_hal {

class MmioHelper {
public:
  explicit MmioHelper(ddk::MmioBuffer *mmio) : mmio_(mmio) {}

  uint32_t Read32(uint32_t offset);
  void Write32(uint32_t offset, uint32_t value);

  void SetBits32(uint32_t offset, uint32_t mask);
  void ClearBits32(uint32_t offset, uint32_t mask);
  void ModifyBits32(uint32_t offset, uint32_t mask, uint32_t value);

  uint32_t ReadMasked32(uint32_t offset, uint32_t mask, uint32_t shift);
  void WriteMasked32(uint32_t offset, uint32_t mask, uint32_t shift,
                     uint32_t value);

  bool WaitForBit32(uint32_t offset, uint32_t bit, bool set,
                    zx::duration timeout);

private:
  ddk::MmioBuffer *mmio_;
};

} // namespace soliloquy_hal

#endif // DRIVERS_COMMON_SOLILOQUY_HAL_MMIO_H_
