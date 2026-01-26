#ifndef DRIVERS_GPU_MALI_G57_MALI_G57_H_
#define DRIVERS_GPU_MALI_G57_MALI_G57_H_

#include <ddktl/device.h>
#include <lib/ddk/device.h>
#include <lib/ddk/driver.h>
#include <lib/mmio/mmio.h>

#include <optional>

#include "registers.h"

namespace mali_g57 {

class MaliG57;
using MaliG57Type = ddk::Device<MaliG57, ddk::Initializable, ddk::Unbindable>;

class MaliG57 : public MaliG57Type {
 public:
  explicit MaliG57(zx_device_t* parent);
  virtual ~MaliG57();

  static zx_status_t Bind(void* ctx, zx_device_t* device);

  void DdkInit(ddk::InitTxn txn);
  void DdkUnbind(ddk::UnbindTxn txn);
  void DdkRelease();

 private:
  zx_status_t Init();
  zx_status_t Shutdown();

  std::optional<ddk::MmioBuffer> gpu_mmio_;
  bool initialized_ = false;

  static constexpr uint32_t kVendorId = 0x13B5;
  static constexpr uint32_t kDeviceId = 0x0B57;
};

}  // namespace mali_g57

#endif  // DRIVERS_GPU_MALI_G57_MALI_G57_H_
