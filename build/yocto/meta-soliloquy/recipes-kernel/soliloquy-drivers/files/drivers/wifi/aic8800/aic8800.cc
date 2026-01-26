#include "aic8800.h"

#include <lib/ddk/debug.h>
#include <lib/ddk/device.h>
#include <lib/ddk/driver.h>
#include <lib/ddk/platform-defs.h>
#include <lib/zx/clock.h>
#include <lib/zx/time.h>
#include <zircon/status.h>
#include <zircon/types.h>

#include <cstring>
#include <memory>

namespace aic8800 {

Aic8800::Aic8800(zx_device_t *parent)
    : Aic8800Type(parent), sdio_(parent), sdio_helper_(&sdio_) {}

Aic8800::~Aic8800() {}

zx_status_t Aic8800::Bind(void *ctx, zx_device_t *device) {
  auto dev = std::make_unique<Aic8800>(device);
  zx_status_t status = dev->DdkAdd("aic8800");
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: Could not create device: %s",
           zx_status_get_string(status));
    return status;
  }
  // dev is now owned by the DDK
  [[maybe_unused]] auto ptr = dev.release();
  return ZX_OK;
}

void Aic8800::DdkInit(ddk::InitTxn txn) {
  zx_status_t status = InitHw();
  txn.Reply(status);
}

void Aic8800::DdkUnbind(ddk::UnbindTxn txn) { txn.Reply(); }

void Aic8800::DdkRelease() { delete this; }

zx_status_t Aic8800::ReadChipId(uint32_t *out_chip_id) {
  uint8_t chip_id_bytes[4];
  for (int i = 0; i < 4; i++) {
    zx_status_t status = sdio_helper_.ReadByte(kRegChipId + i, &chip_id_bytes[i]);
    if (status != ZX_OK) {
      zxlogf(ERROR, "aic8800: Failed to read chip ID byte %d: %s", i,
             zx_status_get_string(status));
      return status;
    }
  }
  
  *out_chip_id = *reinterpret_cast<uint32_t*>(chip_id_bytes);
  
  const char* chip_name = "Unknown";
  if (*out_chip_id == kChipIdAic8800D) {
    chip_name = "AIC8800D";
  } else if (*out_chip_id == kChipIdAic8800Dc) {
    chip_name = "AIC8800DC";
  } else if (*out_chip_id == kChipIdAic8800Dw) {
    chip_name = "AIC8800DW";
  }
  
  zxlogf(INFO, "aic8800: Detected chip: %s (ID: 0x%08x)", chip_name, *out_chip_id);
  return ZX_OK;
}

zx_status_t Aic8800::ResetChip() {
  zxlogf(INFO, "aic8800: Resetting chip...");
  
  zx_status_t status = sdio_helper_.WriteByte(kRegHostCtrl, kHostCtrlReset);
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: Failed to assert reset: %s",
           zx_status_get_string(status));
    return status;
  }
  
  zx::nanosleep(zx::deadline_after(zx::msec(10)));
  
  status = sdio_helper_.WriteByte(kRegHostCtrl, 0);
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: Failed to deassert reset: %s",
           zx_status_get_string(status));
    return status;
  }
  
  zx::nanosleep(zx::deadline_after(zx::msec(50)));
  
  zxlogf(INFO, "aic8800: Reset complete");
  return ZX_OK;
}

zx_status_t Aic8800::SdioFlowControl(uint8_t *out_available_buffers) {
  if (!out_available_buffers) {
    return ZX_ERR_INVALID_ARGS;
  }
  
  for (uint32_t retry = 0; retry < kFlowCtrlRetryCount; retry++) {
    uint8_t fc_reg = 0;
    zx_status_t status = sdio_helper_.ReadByte(kRegFlowCtrl, &fc_reg);
    if (status != ZX_OK) {
      zxlogf(ERROR, "aic8800: Flow control register read failed: %s",
             zx_status_get_string(status));
      return status;
    }
    
    uint8_t available = fc_reg & kFlowCtrlMask;
    if (available != 0) {
      *out_available_buffers = available;
      return ZX_OK;
    }
    
    if (retry < 30) {
      zx::nanosleep(zx::deadline_after(zx::usec(200)));
    } else if (retry < 40) {
      zx::nanosleep(zx::deadline_after(zx::msec(1)));
    } else {
      zx::nanosleep(zx::deadline_after(zx::msec(10)));
    }
  }
  
  zxlogf(ERROR, "aic8800: Flow control timeout - no buffers available");
  return ZX_ERR_TIMED_OUT;
}

zx_status_t Aic8800::SdioTx(const uint8_t *buf, size_t len, uint8_t func_num) {
  if (!buf || len == 0) {
    return ZX_ERR_INVALID_ARGS;
  }
  
  size_t aligned_len = (len + kBlockSize - 1) / kBlockSize * kBlockSize;
  
  uint8_t available_buffers = 0;
  zx_status_t status = SdioFlowControl(&available_buffers);
  if (status != ZX_OK) {
    return status;
  }
  
  size_t required_buffers = (aligned_len + kBufferSize - 1) / kBufferSize;
  if (available_buffers < required_buffers) {
    zxlogf(ERROR, "aic8800: Insufficient buffers for TX: need %zu, have %u",
           required_buffers, available_buffers);
    return ZX_ERR_NO_RESOURCES;
  }
  
  status = sdio_helper_.WriteMultiBlock(func_num, buf, aligned_len);
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: SDIO TX failed (func %u, len %zu): %s",
           func_num, aligned_len, zx_status_get_string(status));
    return status;
  }
  
  return ZX_OK;
}

zx_status_t Aic8800::SdioRx(uint8_t *buf, size_t len, uint8_t func_num) {
  if (!buf || len == 0) {
    return ZX_ERR_INVALID_ARGS;
  }
  
  size_t aligned_len = (len + kBlockSize - 1) / kBlockSize * kBlockSize;
  
  zx_status_t status = sdio_helper_.ReadMultiBlock(func_num, buf, aligned_len);
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: SDIO RX failed (func %u, len %zu): %s",
           func_num, aligned_len, zx_status_get_string(status));
    return status;
  }
  
  return ZX_OK;
}

zx_status_t Aic8800::WaitForFirmwareReady() {
  zxlogf(INFO, "aic8800: Waiting for firmware ready...");
  
  auto deadline = zx::deadline_after(zx::msec(kFwReadyTimeoutMs));
  
  while (zx::clock::get_monotonic() < deadline) {
    uint8_t fw_status;
    zx_status_t status = sdio_helper_.ReadByte(kRegFwStatus, &fw_status);
    if (status != ZX_OK) {
      zxlogf(ERROR, "aic8800: Failed to read firmware status: %s",
             zx_status_get_string(status));
      return status;
    }
    
    if (fw_status == kFwStatusReady) {
      zxlogf(INFO, "aic8800: Firmware ready");
      return ZX_OK;
    }
    
    if (fw_status == kFwStatusError) {
      zxlogf(ERROR, "aic8800: Firmware reported error status");
      return ZX_ERR_INTERNAL;
    }
    
    zx::nanosleep(zx::deadline_after(zx::msec(100)));
  }
  
  zxlogf(ERROR, "aic8800: Timeout waiting for firmware ready");
  return ZX_ERR_TIMED_OUT;
}

zx_status_t Aic8800::ConfigurePatchTables() {
  zxlogf(INFO, "aic8800: Configuring patch tables...");
  
  static const PatchEntry kPatchTable8800D80[] = {
    {0x00b4, 0xf3010000},
    {0x0170, 0x0001000A},
  };
  
  constexpr uint32_t kConfigBaseAddr = kRamFmacFwAddrU02 + 0x0198;
  constexpr uint32_t kPatchStrBaseAddr = kRamFmacFwAddrU02 + 0x01A0;
  
  uint32_t config_base = 0;
  uint8_t config_bytes[4];
  for (int i = 0; i < 4; i++) {
    zx_status_t status = sdio_helper_.ReadByte(kConfigBaseAddr + i, &config_bytes[i]);
    if (status != ZX_OK) {
      zxlogf(ERROR, "aic8800: Failed to read config base address: %s",
             zx_status_get_string(status));
      return status;
    }
  }
  config_base = *reinterpret_cast<uint32_t*>(config_bytes);
  
  uint32_t patch_str_base = 0;
  uint8_t patch_str_bytes[4];
  for (int i = 0; i < 4; i++) {
    zx_status_t status = sdio_helper_.ReadByte(kPatchStrBaseAddr + i, &patch_str_bytes[i]);
    if (status != ZX_OK) {
      zxlogf(ERROR, "aic8800: Failed to read patch string base address: %s",
             zx_status_get_string(status));
      return status;
    }
  }
  patch_str_base = *reinterpret_cast<uint32_t*>(patch_str_bytes);
  
  zxlogf(INFO, "aic8800: Config base: 0x%08x, Patch str base: 0x%08x",
         config_base, patch_str_base);
  
  auto write_u32 = [this](uint32_t addr, uint32_t value) -> zx_status_t {
    uint8_t bytes[4];
    bytes[0] = value & 0xFF;
    bytes[1] = (value >> 8) & 0xFF;
    bytes[2] = (value >> 16) & 0xFF;
    bytes[3] = (value >> 24) & 0xFF;
    for (int i = 0; i < 4; i++) {
      zx_status_t status = sdio_helper_.WriteByte(addr + i, bytes[i]);
      if (status != ZX_OK) {
        return status;
      }
    }
    return ZX_OK;
  };
  
  constexpr size_t kPatchOfstMagicNum = 0;
  constexpr size_t kPatchOfstPairStart = 4;
  constexpr size_t kPatchOfstMagicNum2 = 8;
  constexpr size_t kPatchOfstPairCount = 12;
  constexpr size_t kPatchOfstBlockSize = 32;
  
  zx_status_t status = write_u32(patch_str_base + kPatchOfstMagicNum, kPatchMagicNum);
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: Failed to write patch magic number");
    return status;
  }
  
  status = write_u32(patch_str_base + kPatchOfstMagicNum2, kPatchMagicNum2);
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: Failed to write patch magic number 2");
    return status;
  }
  
  status = write_u32(patch_str_base + kPatchOfstPairStart, kPatchStartAddr);
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: Failed to write patch pair start");
    return status;
  }
  
  size_t patch_count = sizeof(kPatchTable8800D80) / sizeof(PatchEntry);
  status = write_u32(patch_str_base + kPatchOfstPairCount, patch_count);
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: Failed to write patch pair count");
    return status;
  }
  
  for (size_t i = 0; i < patch_count; i++) {
    uint32_t entry_addr = kPatchStartAddr + (i * 8);
    status = write_u32(entry_addr, kPatchTable8800D80[i].offset + config_base);
    if (status != ZX_OK) {
      zxlogf(ERROR, "aic8800: Failed to write patch entry %zu offset", i);
      return status;
    }
    status = write_u32(entry_addr + 4, kPatchTable8800D80[i].value);
    if (status != ZX_OK) {
      zxlogf(ERROR, "aic8800: Failed to write patch entry %zu value", i);
      return status;
    }
  }
  
  for (int i = 0; i < 4; i++) {
    status = write_u32(patch_str_base + kPatchOfstBlockSize + (i * 4), 0);
    if (status != ZX_OK) {
      zxlogf(ERROR, "aic8800: Failed to write block size %d", i);
      return status;
    }
  }
  
  zxlogf(INFO, "aic8800: Patch configuration complete (%zu entries)", patch_count);
  return ZX_OK;
}

zx_status_t Aic8800::InitHw() {
  zxlogf(INFO, "aic8800: Initializing hardware...");

  zx_status_t status = ReadChipId(&chip_id_);
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: Failed to read chip ID: %s",
           zx_status_get_string(status));
    return status;
  }
  
  if (chip_id_ != kChipIdAic8800D && chip_id_ != kChipIdAic8800Dc && 
      chip_id_ != kChipIdAic8800Dw) {
    zxlogf(ERROR, "aic8800: Unsupported chip ID: 0x%08x", chip_id_);
    return ZX_ERR_NOT_SUPPORTED;
  }
  
  status = ResetChip();
  if (status != ZX_OK) {
    return status;
  }

  const char *kFwName = "fmacfw_8800d80.bin";

  zx::vmo fw_vmo;
  size_t fw_size;
  status = soliloquy_hal::FirmwareLoader::LoadFirmware(
      parent(), kFwName, &fw_vmo, &fw_size);
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: Failed to load firmware '%s': %s", kFwName,
           zx_status_get_string(status));
    return status;
  }
  
  if (fw_size > kFirmwareMaxSize) {
    zxlogf(ERROR, "aic8800: Firmware too large: %zu bytes (max %zu)", 
           fw_size, kFirmwareMaxSize);
    return ZX_ERR_BUFFER_TOO_SMALL;
  }

  status = sdio_helper_.DownloadFirmware(fw_vmo, fw_size, kRamFmacFwAddrU02);
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: Failed to download firmware: %s",
           zx_status_get_string(status));
    return status;
  }
  
  status = ConfigurePatchTables();
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: Failed to configure patch tables: %s",
           zx_status_get_string(status));
    return status;
  }
  
  status = WaitForFirmwareReady();
  if (status != ZX_OK) {
    return status;
  }
  
  status = sdio_helper_.WriteByte(kRegHostCtrl, kHostCtrlEnable);
  if (status != ZX_OK) {
    zxlogf(ERROR, "aic8800: Failed to enable chip: %s",
           zx_status_get_string(status));
    return status;
  }

  initialized_ = true;
  zxlogf(INFO, "aic8800: Hardware initialization complete");
  return ZX_OK;
}

zx_status_t Aic8800::WlanphyImplQuery(wlanphy_info_t *out_info) {
  if (!initialized_) {
    zxlogf(ERROR, "aic8800: Device not initialized");
    return ZX_ERR_BAD_STATE;
  }
  
  if (!out_info) {
    return ZX_ERR_INVALID_ARGS;
  }
  
  memset(out_info, 0, sizeof(*out_info));
  
  out_info->supported_phys = WLAN_INFO_PHY_TYPE_DSSS | WLAN_INFO_PHY_TYPE_CCK |
                              WLAN_INFO_PHY_TYPE_OFDM | WLAN_INFO_PHY_TYPE_HT;
  
  out_info->driver_features = 0;
  
  out_info->mac_modes = WLAN_INFO_MAC_MODE_STA | WLAN_INFO_MAC_MODE_AP;
  
  out_info->caps = WLAN_INFO_HARDWARE_CAPABILITY_SHORT_PREAMBLE |
                   WLAN_INFO_HARDWARE_CAPABILITY_SHORT_SLOT_TIME;
  
  out_info->bands_count = 1;
  
  auto &band = out_info->bands[0];
  band.band = WLAN_INFO_BAND_2GHZ;
  
  band.ht_supported = true;
  band.ht_caps.ht_capability_info = 0x016E;
  band.ht_caps.ampdu_params = 0x17;
  
  static const uint8_t kSupportedMcs[] = {
      0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
  };
  memcpy(band.ht_caps.supported_mcs_set, kSupportedMcs, sizeof(kSupportedMcs));
  
  band.vht_supported = false;
  
  static const uint8_t kChannels2g[] = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13};
  band.supported_channels.base_freq = 2407;
  band.supported_channels.channels_count = sizeof(kChannels2g);
  memcpy(band.supported_channels.channels, kChannels2g, sizeof(kChannels2g));
  
  zxlogf(INFO, "aic8800: WlanphyQuery - PHY: 0x%x, MAC modes: 0x%x, Bands: %u",
         out_info->supported_phys, out_info->mac_modes, out_info->bands_count);
  
  return ZX_OK;
}

zx_status_t
Aic8800::WlanphyImplCreateIface(const wlanphy_create_iface_req_t *req,
                                uint16_t *out_iface_id) {
  if (!initialized_) {
    return ZX_ERR_BAD_STATE;
  }
  
  if (!req || !out_iface_id) {
    return ZX_ERR_INVALID_ARGS;
  }
  
  zxlogf(INFO, "aic8800: CreateIface requested - role: %u", req->role);
  
  return ZX_ERR_NOT_SUPPORTED;
}

zx_status_t Aic8800::WlanphyImplDestroyIface(uint16_t iface_id) {
  if (!initialized_) {
    return ZX_ERR_BAD_STATE;
  }
  
  zxlogf(INFO, "aic8800: DestroyIface requested - ID: %u", iface_id);
  
  return ZX_ERR_NOT_SUPPORTED;
}

zx_status_t Aic8800::WlanphyImplSetCountry(const wlanphy_country_t *country) {
  if (!initialized_) {
    return ZX_ERR_BAD_STATE;
  }
  
  if (!country) {
    return ZX_ERR_INVALID_ARGS;
  }
  
  zxlogf(INFO, "aic8800: SetCountry requested - code: %.2s", country->alpha2);
  
  return ZX_ERR_NOT_SUPPORTED;
}

zx_status_t Aic8800::WlanphyImplClearCountry() {
  if (!initialized_) {
    return ZX_ERR_BAD_STATE;
  }
  
  zxlogf(INFO, "aic8800: ClearCountry requested");
  
  return ZX_ERR_NOT_SUPPORTED;
}

zx_status_t Aic8800::WlanphyImplGetCountry(wlanphy_country_t *out_country) {
  if (!initialized_) {
    return ZX_ERR_BAD_STATE;
  }
  
  if (!out_country) {
    return ZX_ERR_INVALID_ARGS;
  }
  
  zxlogf(INFO, "aic8800: GetCountry requested");
  
  return ZX_ERR_NOT_SUPPORTED;
}

static constexpr zx_driver_ops_t aic8800_driver_ops = []() {
  zx_driver_ops_t ops = {};
  ops.version = DRIVER_OPS_VERSION;
  ops.bind = Aic8800::Bind;
  return ops;
}();

} // namespace aic8800

ZIRCON_DRIVER(aic8800, aic8800::aic8800_driver_ops, "zircon", "0.1");
