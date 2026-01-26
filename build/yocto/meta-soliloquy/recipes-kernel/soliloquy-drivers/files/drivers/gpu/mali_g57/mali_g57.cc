#include "mali_g57.h"

#include <lib/ddk/debug.h>
#include <lib/ddk/device.h>
#include <lib/ddk/driver.h>
#include <lib/ddk/platform-defs.h>
#include <zircon/status.h>
#include <zircon/types.h>

#include <memory>

namespace mali_g57 {

MaliG57::MaliG57(zx_device_t* parent) : MaliG57Type(parent) {}

MaliG57::~MaliG57() {
  if (initialized_) {
    Shutdown();
  }
}

zx_status_t MaliG57::Bind(void* ctx, zx_device_t* device) {
  auto dev = std::make_unique<MaliG57>(device);
  zx_status_t status = dev->DdkAdd("mali-g57");
  if (status != ZX_OK) {
    zxlogf(ERROR, "mali-g57: Could not create device: %s",
           zx_status_get_string(status));
    return status;
  }
  [[maybe_unused]] auto ptr = dev.release();
  return ZX_OK;
}

void MaliG57::DdkInit(ddk::InitTxn txn) {
  zx_status_t status = Init();
  txn.Reply(status);
}

void MaliG57::DdkUnbind(ddk::UnbindTxn txn) {
  Shutdown();
  txn.Reply();
}

void MaliG57::DdkRelease() { delete this; }

zx_status_t MaliG57::Init() {
  zxlogf(INFO, "Mali-G57 Driver Loaded");
  zxlogf(INFO, "mali-g57: Initializing hardware...");
  zxlogf(INFO, "mali-g57: Vendor ID: 0x%04X, Device ID: 0x%04X", kVendorId,
         kDeviceId);

  initialized_ = true;
  zxlogf(INFO, "mali-g57: Initialization complete");
  return ZX_OK;
}

zx_status_t MaliG57::Shutdown() {
  if (!initialized_) {
    return ZX_OK;
  }

  zxlogf(INFO, "mali-g57: Shutting down...");

  if (gpu_mmio_.has_value()) {
    gpu_mmio_.reset();
  }

  initialized_ = false;
  zxlogf(INFO, "mali-g57: Shutdown complete");
  return ZX_OK;
}

static constexpr zx_driver_ops_t mali_g57_driver_ops = []() {
  zx_driver_ops_t ops = {};
  ops.version = DRIVER_OPS_VERSION;
  ops.bind = MaliG57::Bind;
  return ops;
}();

}  // namespace mali_g57

ZIRCON_DRIVER(mali_g57, mali_g57::mali_g57_driver_ops, "zircon", "0.1");
