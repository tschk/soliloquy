#include "gpio.h"

#include <lib/ddk/debug.h>
#include <lib/ddk/device.h>
#include <lib/ddk/driver.h>
#include <lib/ddk/platform-defs.h>
#include <zircon/status.h>
#include <zircon/types.h>

#include <memory>

namespace soliloquy_gpio {

SoliloquyGpio::SoliloquyGpio(zx_device_t *parent) : SoliloquyGpioType(parent) {}

SoliloquyGpio::~SoliloquyGpio() {}

zx_status_t SoliloquyGpio::Bind(void *ctx, zx_device_t *device) {
  auto dev = std::make_unique<SoliloquyGpio>(device);
  zx_status_t status = dev->DdkAdd("soliloquy-gpio");
  if (status != ZX_OK) {
    zxlogf(ERROR, "soliloquy-gpio: Could not create device: %s",
           zx_status_get_string(status));
    return status;
  }
  [[maybe_unused]] auto ptr = dev.release();
  return ZX_OK;
}

void SoliloquyGpio::DdkInit(ddk::InitTxn txn) {
  zx_status_t status = InitHw();
  txn.Reply(status);
}

void SoliloquyGpio::DdkUnbind(ddk::UnbindTxn txn) { txn.Reply(); }

void SoliloquyGpio::DdkRelease() { delete this; }

zx_status_t SoliloquyGpio::InitHw() {
  zxlogf(INFO, "soliloquy-gpio: Initializing GPIO controller...");

  zx_status_t status =
      ddk::MmioBuffer::Create(kGpioBaseAddr, kGpioMmioSize, zx::resource(),
                              ZX_CACHE_POLICY_UNCACHED_DEVICE, &gpio_mmio_);

  if (status != ZX_OK) {
    zxlogf(ERROR, "soliloquy-gpio: Failed to map GPIO MMIO: %s",
           zx_status_get_string(status));
    return status;
  }

  mmio_helper_ =
      std::make_unique<soliloquy_hal::MmioHelper>(&gpio_mmio_.value());

  zxlogf(INFO, "soliloquy-gpio: GPIO controller initialized");
  return ZX_OK;
}

zx_status_t SoliloquyGpio::GpioConfigIn(uint32_t flags) {
  if (!mmio_helper_) {
    return ZX_ERR_BAD_STATE;
  }

  mmio_helper_->ClearBits32(kGpioDirReg, 1);

  constexpr uint32_t GPIO_PULL_UP = 0x1;
  constexpr uint32_t GPIO_PULL_DOWN = 0x2;

  if (flags & GPIO_PULL_UP) {
    mmio_helper_->SetBits32(kGpioPullReg, 0x1);
  } else if (flags & GPIO_PULL_DOWN) {
    mmio_helper_->SetBits32(kGpioPullReg, 0x2);
  } else {
    mmio_helper_->ClearBits32(kGpioPullReg, 0x3);
  }

  zxlogf(DEBUG, "soliloquy-gpio: Configured pin as input");
  return ZX_OK;
}

zx_status_t SoliloquyGpio::GpioConfigOut(uint8_t initial_value) {
  if (!mmio_helper_) {
    return ZX_ERR_BAD_STATE;
  }

  mmio_helper_->SetBits32(kGpioDirReg, 1);

  if (initial_value) {
    mmio_helper_->SetBits32(kGpioDataReg, 1);
  } else {
    mmio_helper_->ClearBits32(kGpioDataReg, 1);
  }

  zxlogf(DEBUG, "soliloquy-gpio: Configured pin as output");
  return ZX_OK;
}

zx_status_t SoliloquyGpio::GpioSetAltFunction(uint64_t function) {
  zxlogf(DEBUG, "soliloquy-gpio: Setting alt function %lu", function);
  return ZX_OK;
}

zx_status_t SoliloquyGpio::GpioRead(uint8_t *out_value) {
  if (!mmio_helper_ || !out_value) {
    return ZX_ERR_INVALID_ARGS;
  }

  uint32_t val = mmio_helper_->Read32(kGpioDataReg);
  *out_value = (val & 1) ? 1 : 0;

  return ZX_OK;
}

zx_status_t SoliloquyGpio::GpioWrite(uint8_t value) {
  if (!mmio_helper_) {
    return ZX_ERR_BAD_STATE;
  }

  if (value) {
    mmio_helper_->SetBits32(kGpioDataReg, 1);
  } else {
    mmio_helper_->ClearBits32(kGpioDataReg, 1);
  }

  return ZX_OK;
}

zx_status_t SoliloquyGpio::GpioGetInterrupt(uint32_t flags,
                                            zx::interrupt *out_irq) {
  return ZX_ERR_NOT_SUPPORTED;
}

zx_status_t SoliloquyGpio::GpioReleaseInterrupt() {
  return ZX_ERR_NOT_SUPPORTED;
}

zx_status_t SoliloquyGpio::GpioSetPolarity(gpio_polarity_t polarity) {
  return ZX_ERR_NOT_SUPPORTED;
}

static constexpr zx_driver_ops_t soliloquy_gpio_driver_ops = []() {
  zx_driver_ops_t ops = {};
  ops.version = DRIVER_OPS_VERSION;
  ops.bind = SoliloquyGpio::Bind;
  return ops;
}();

} // namespace soliloquy_gpio

ZIRCON_DRIVER(soliloquy_gpio, soliloquy_gpio::soliloquy_gpio_driver_ops,
              "zircon", "0.1");
