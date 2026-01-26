#ifndef SRC_CONNECTIVITY_WLAN_DRIVERS_THIRD_PARTY_AIC8800_AIC8800_H_
#define SRC_CONNECTIVITY_WLAN_DRIVERS_THIRD_PARTY_AIC8800_AIC8800_H_

#include <ddktl/device.h>
#include <ddktl/protocol/wlanphyimpl.h>
#include <fuchsia/hardware/sdio/cpp/banjo.h>
#include <lib/ddk/device.h>
#include <lib/ddk/driver.h>

#include "../../common/soliloquy_hal/firmware.h"
#include "../../common/soliloquy_hal/sdio.h"

namespace aic8800 {

class Aic8800;
using Aic8800Type = ddk::Device<Aic8800, ddk::Initializable, ddk::Unbindable>;

class Aic8800 : public Aic8800Type,
                public ddk::WlanphyImplProtocol<Aic8800, ddk::base_protocol> {
public:
  explicit Aic8800(zx_device_t *parent);
  virtual ~Aic8800();

  static zx_status_t Bind(void *ctx, zx_device_t *device);

  // DDK Lifecycle methods
  void DdkInit(ddk::InitTxn txn);
  void DdkUnbind(ddk::UnbindTxn txn);
  void DdkRelease();

  // WlanphyImplProtocol methods
  zx_status_t WlanphyImplQuery(wlanphy_info_t *out_info);
  zx_status_t WlanphyImplCreateIface(const wlanphy_create_iface_req_t *req,
                                     uint16_t *out_iface_id);
  zx_status_t WlanphyImplDestroyIface(uint16_t iface_id);
  zx_status_t WlanphyImplSetCountry(const wlanphy_country_t *country);
  zx_status_t WlanphyImplClearCountry();
  zx_status_t WlanphyImplGetCountry(wlanphy_country_t *out_country);

private:
  zx_status_t InitHw();
  zx_status_t ReadChipId(uint32_t *out_chip_id);
  zx_status_t WaitForFirmwareReady();
  zx_status_t ResetChip();
  zx_status_t ConfigurePatchTables();
  
  zx_status_t SdioTx(const uint8_t *buf, size_t len, uint8_t func_num);
  zx_status_t SdioRx(uint8_t *buf, size_t len, uint8_t func_num);
  zx_status_t SdioFlowControl(uint8_t *out_available_buffers);

  ddk::SdioProtocolClient sdio_;
  soliloquy_hal::SdioHelper sdio_helper_;
  
  uint32_t chip_id_ = 0;
  bool initialized_ = false;

  static constexpr uint32_t kVendorId = 0xA5C8;
  static constexpr uint32_t kDeviceId = 0x8800;
  
  static constexpr uint32_t kRegChipId = 0x00000000;
  static constexpr uint32_t kRegChipRev = 0x00000004;
  static constexpr uint32_t kRegFwStatus = 0x00000008;
  static constexpr uint32_t kRegHostCtrl = 0x0000000C;
  static constexpr uint32_t kRegIntStatus = 0x00000010;
  static constexpr uint32_t kRegIntMask = 0x00000014;
  static constexpr uint32_t kRegTxReady = 0x00000018;
  static constexpr uint32_t kRegRxReady = 0x0000001C;
  
  static constexpr uint32_t kRegSdioCtrl = 0x00000100;
  static constexpr uint32_t kRegBlockSize = 0x00000110;
  static constexpr uint32_t kRegBlockCount = 0x00000114;
  
  static constexpr uint32_t kRegFwDownloadAddr = 0x00100000;
  static constexpr uint32_t kRegFwDownloadSize = 0x00100004;
  static constexpr uint32_t kRegFwDownloadCtrl = 0x00100008;
  
  static constexpr uint8_t kRegByteModeLen = 0x02;
  static constexpr uint8_t kRegSleepCtrl = 0x05;
  static constexpr uint8_t kRegWakeup = 0x09;
  static constexpr uint8_t kRegFlowCtrl = 0x0A;
  
  static constexpr uint8_t kFlowCtrlMask = 0x7F;
  static constexpr uint32_t kFlowCtrlRetryCount = 50;
  static constexpr uint32_t kBufferSize = 1536;
  
  static constexpr uint32_t kIntFwReady = 1 << 0;
  static constexpr uint32_t kIntTxDone = 1 << 1;
  static constexpr uint32_t kIntRxReady = 1 << 2;
  static constexpr uint32_t kIntError = 1 << 31;
  
  static constexpr uint32_t kHostCtrlReset = 1 << 0;
  static constexpr uint32_t kHostCtrlEnable = 1 << 1;
  static constexpr uint32_t kHostCtrlSleep = 1 << 2;
  
  static constexpr uint32_t kFwStatusIdle = 0;
  static constexpr uint32_t kFwStatusDownloading = 1;
  static constexpr uint32_t kFwStatusReady = 2;
  static constexpr uint32_t kFwStatusError = 0xFF;
  
  static constexpr uint32_t kChipIdAic8800D = 0x88000000;
  static constexpr uint32_t kChipIdAic8800Dc = 0x88000001;
  static constexpr uint32_t kChipIdAic8800Dw = 0x88000002;
  
  static constexpr uint32_t kFirmwareBaseAddr = 0x00100000;
  static constexpr size_t kFirmwareMaxSize = 512 * 1024;
  
  static constexpr size_t kBlockSize = 512;
  static constexpr int kFwReadyTimeoutMs = 5000;
  
  static constexpr uint32_t kRamFmacFwAddrU02 = 0x00120000;
  static constexpr uint32_t kPatchMagicNum = 0x48435450;
  static constexpr uint32_t kPatchMagicNum2 = 0x50544348;
  static constexpr uint32_t kPatchStartAddr = 0x001D7000;
  
  struct PatchEntry {
    uint32_t offset;
    uint32_t value;
  };
};

} // namespace aic8800

#endif // SRC_CONNECTIVITY_WLAN_DRIVERS_THIRD_PARTY_AIC8800_AIC8800_H_
