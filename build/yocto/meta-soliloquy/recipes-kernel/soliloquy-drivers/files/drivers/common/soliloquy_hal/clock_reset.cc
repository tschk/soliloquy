#include "clock_reset.h"

#include <lib/ddk/debug.h>
#include <zircon/status.h>

namespace soliloquy_hal {

zx_status_t ClockResetHelper::EnableClock(uint32_t clock_id) {
  if (!ccu_mmio_) {
    return ZX_ERR_BAD_STATE;
  }

  uint32_t reg_offset = kClockGateReg + (clock_id / 32) * 4;
  uint32_t bit_offset = clock_id % 32;

  uint32_t val = ccu_mmio_->Read32(reg_offset);
  val |= (1 << bit_offset);
  ccu_mmio_->Write32(val, reg_offset);

  zxlogf(DEBUG, "soliloquy_hal: Enabled clock %u", clock_id);
  return ZX_OK;
}

zx_status_t ClockResetHelper::DisableClock(uint32_t clock_id) {
  if (!ccu_mmio_) {
    return ZX_ERR_BAD_STATE;
  }

  uint32_t reg_offset = kClockGateReg + (clock_id / 32) * 4;
  uint32_t bit_offset = clock_id % 32;

  uint32_t val = ccu_mmio_->Read32(reg_offset);
  val &= ~(1 << bit_offset);
  ccu_mmio_->Write32(val, reg_offset);

  zxlogf(DEBUG, "soliloquy_hal: Disabled clock %u", clock_id);
  return ZX_OK;
}

zx_status_t ClockResetHelper::AssertReset(uint32_t reset_id) {
  if (!ccu_mmio_) {
    return ZX_ERR_BAD_STATE;
  }

  uint32_t reg_offset = kResetReg + (reset_id / 32) * 4;
  uint32_t bit_offset = reset_id % 32;

  uint32_t val = ccu_mmio_->Read32(reg_offset);
  val &= ~(1 << bit_offset);
  ccu_mmio_->Write32(val, reg_offset);

  zxlogf(DEBUG, "soliloquy_hal: Asserted reset %u", reset_id);
  return ZX_OK;
}

zx_status_t ClockResetHelper::DeassertReset(uint32_t reset_id) {
  if (!ccu_mmio_) {
    return ZX_ERR_BAD_STATE;
  }

  uint32_t reg_offset = kResetReg + (reset_id / 32) * 4;
  uint32_t bit_offset = reset_id % 32;

  uint32_t val = ccu_mmio_->Read32(reg_offset);
  val |= (1 << bit_offset);
  ccu_mmio_->Write32(val, reg_offset);

  zxlogf(DEBUG, "soliloquy_hal: Deasserted reset %u", reset_id);
  return ZX_OK;
}

zx_status_t ClockResetHelper::SetClockRate(uint32_t clock_id,
                                           uint64_t rate_hz) {
  if (!ccu_mmio_) {
    return ZX_ERR_BAD_STATE;
  }

  zxlogf(INFO, "soliloquy_hal: Setting clock %u rate to %lu Hz", clock_id,
         rate_hz);
  return ZX_ERR_NOT_SUPPORTED;
}

zx_status_t ClockResetHelper::GetClockRate(uint32_t clock_id,
                                           uint64_t *out_rate_hz) {
  if (!ccu_mmio_ || !out_rate_hz) {
    return ZX_ERR_INVALID_ARGS;
  }

  *out_rate_hz = 0;
  return ZX_ERR_NOT_SUPPORTED;
}

} // namespace soliloquy_hal
