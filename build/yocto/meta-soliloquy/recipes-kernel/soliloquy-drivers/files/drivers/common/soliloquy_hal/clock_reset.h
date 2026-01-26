#ifndef DRIVERS_COMMON_SOLILOQUY_HAL_CLOCK_RESET_H_
#define DRIVERS_COMMON_SOLILOQUY_HAL_CLOCK_RESET_H_

#include <lib/mmio/mmio.h>
#include <zircon/types.h>

namespace soliloquy_hal {

class ClockResetHelper {
public:
  explicit ClockResetHelper(ddk::MmioBuffer *ccu_mmio) : ccu_mmio_(ccu_mmio) {}

  zx_status_t EnableClock(uint32_t clock_id);
  zx_status_t DisableClock(uint32_t clock_id);

  zx_status_t AssertReset(uint32_t reset_id);
  zx_status_t DeassertReset(uint32_t reset_id);

  zx_status_t SetClockRate(uint32_t clock_id, uint64_t rate_hz);
  zx_status_t GetClockRate(uint32_t clock_id, uint64_t *out_rate_hz);

private:
  ddk::MmioBuffer *ccu_mmio_;

  static constexpr uint32_t kClockGateReg = 0x0000;
  static constexpr uint32_t kResetReg = 0x0100;
  static constexpr uint32_t kClockConfigReg = 0x0200;
};

} // namespace soliloquy_hal

#endif // DRIVERS_COMMON_SOLILOQUY_HAL_CLOCK_RESET_H_
