#ifndef DRIVERS_GPIO_SOLILOQUY_GPIO_GPIO_H_
#define DRIVERS_GPIO_SOLILOQUY_GPIO_GPIO_H_

#include <ddktl/device.h>
#include <fuchsia/hardware/gpio/cpp/banjo.h>
#include <lib/ddk/device.h>
#include <lib/ddk/driver.h>
#include <lib/mmio/mmio.h>

#include "../../common/soliloquy_hal/mmio.h"

namespace soliloquy_gpio {

class SoliloquyGpio;
using SoliloquyGpioType =
    ddk::Device<SoliloquyGpio, ddk::Initializable, ddk::Unbindable>;

class SoliloquyGpio
    : public SoliloquyGpioType,
      public ddk::GpioProtocol<SoliloquyGpio, ddk::base_protocol> {
public:
  explicit SoliloquyGpio(zx_device_t *parent);
  virtual ~SoliloquyGpio();

  static zx_status_t Bind(void *ctx, zx_device_t *device);

  void DdkInit(ddk::InitTxn txn);
  void DdkUnbind(ddk::UnbindTxn txn);
  void DdkRelease();

  zx_status_t GpioConfigIn(uint32_t flags);
  zx_status_t GpioConfigOut(uint8_t initial_value);
  zx_status_t GpioSetAltFunction(uint64_t function);
  zx_status_t GpioRead(uint8_t *out_value);
  zx_status_t GpioWrite(uint8_t value);
  zx_status_t GpioGetInterrupt(uint32_t flags, zx::interrupt *out_irq);
  zx_status_t GpioReleaseInterrupt();
  zx_status_t GpioSetPolarity(gpio_polarity_t polarity);

private:
  zx_status_t InitHw();

  std::optional<ddk::MmioBuffer> gpio_mmio_;
  std::unique_ptr<soliloquy_hal::MmioHelper> mmio_helper_;

  static constexpr uint32_t kGpioBaseAddr = 0x01C20800;
  static constexpr size_t kGpioMmioSize = 0x400;

  static constexpr uint32_t kGpioDataReg = 0x10;
  static constexpr uint32_t kGpioDirReg = 0x00;
  static constexpr uint32_t kGpioPullReg = 0x1C;
};

} // namespace soliloquy_gpio

#endif // DRIVERS_GPIO_SOLILOQUY_GPIO_GPIO_H_
