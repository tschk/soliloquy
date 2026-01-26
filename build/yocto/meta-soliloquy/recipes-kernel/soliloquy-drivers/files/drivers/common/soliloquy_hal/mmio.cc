#include "mmio.h"

#include <lib/ddk/debug.h>
#include <zircon/time.h>

namespace soliloquy_hal {

uint32_t MmioHelper::Read32(uint32_t offset) { return mmio_->Read32(offset); }

// Writes a 32-bit value to a memory-mapped hardware register.
// Assumes: 32-bit aligned access, write-through semantics (no buffering
// required).
void MmioHelper::Write32(uint32_t offset, uint32_t value) {
  mmio_->Write32(value, offset);
}

// Sets specific bits in a register using bitwise OR (read-modify-write).
// Operation: reg[offset] = reg[offset] | mask
// Use case: Enable interrupt flags, set control bits without affecting others.
void MmioHelper::SetBits32(uint32_t offset, uint32_t mask) {
  uint32_t val = Read32(offset);
  Write32(offset, val | mask);
}

// Clears specific bits in a register using bitwise AND with inverted mask.
// Operation: reg[offset] = reg[offset] & ~mask
// Use case: Disable interrupts, clear status flags.
void MmioHelper::ClearBits32(uint32_t offset, uint32_t mask) {
  uint32_t val = Read32(offset);
  Write32(offset, val & ~mask);
}

// Modifies specific bits in a register while preserving others.
// Operation: reg[offset] = (reg[offset] & ~mask) | (value & mask)
// Use case: Update multi-bit fields (e.g., set clock divider, configure mode
// bits).
void MmioHelper::ModifyBits32(uint32_t offset, uint32_t mask, uint32_t value) {
  uint32_t val = Read32(offset);
  val = (val & ~mask) | (value & mask);
  Write32(offset, val);
}

// Reads a specific bit field from a register, applying mask and shift.
// Operation: (reg[offset] & mask) >> shift
// Use case: Extract status bits or multi-bit configuration values.
// Example: ReadMasked32(0x10, 0x0F00, 8) reads bits [11:8].
uint32_t MmioHelper::ReadMasked32(uint32_t offset, uint32_t mask,
                                  uint32_t shift) {
  return (Read32(offset) & mask) >> shift;
}

// Writes a value to a specific bit field in a register, applying shift and
// mask. Operation: reg[offset] = (reg[offset] & ~mask) | ((value << shift) &
// mask) Use case: Update multi-bit fields without affecting other bits.
// Example: WriteMasked32(0x10, 0x0F00, 8, 5) writes 5 to bits [11:8].
void MmioHelper::WriteMasked32(uint32_t offset, uint32_t mask, uint32_t shift,
                               uint32_t value) {
  uint32_t val = Read32(offset);
  val = (val & ~mask) | ((value << shift) & mask);
  Write32(offset, val);
}

// Polls a register bit until it reaches the expected state or timeout expires.
// Polling interval: 10 microseconds (hardware assumption: status updates at
// ~1kHz or faster). Use case: Wait for hardware initialization, DMA completion,
// or status flags. Returns: true if bit reached expected state, false on
// timeout (warning logged). Hardware assumptions:
// - Register reads are idempotent (no side effects from repeated reads)
// - Status updates occur within microseconds to milliseconds
// - 10µs polling granularity is sufficient for most hardware state machines
bool MmioHelper::WaitForBit32(uint32_t offset, uint32_t bit, bool set,
                              zx::duration timeout) {
  zx_time_t deadline = zx_deadline_after(timeout.get());

  while (zx_clock_get_monotonic() < deadline) {
    uint32_t val = Read32(offset);
    bool bit_set = (val & (1 << bit)) != 0;

    if (bit_set == set) {
      return true;
    }

    zx_nanosleep(zx_deadline_after(ZX_USEC(10)));
  }

  zxlogf(WARNING, "soliloquy_hal: Timeout waiting for bit %u at offset 0x%x",
         bit, offset);
  return false;
}

} // namespace soliloquy_hal
