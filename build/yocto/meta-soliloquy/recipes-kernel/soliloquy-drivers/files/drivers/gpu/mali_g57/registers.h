#ifndef DRIVERS_GPU_MALI_G57_REGISTERS_H_
#define DRIVERS_GPU_MALI_G57_REGISTERS_H_

#include <cstdint>

namespace mali_g57 {

constexpr uint64_t kMaliBaseAddr = 0x01800000;
constexpr size_t kMaliMmioSize = 0x10000;

constexpr uint32_t kGpuControlBase = 0x00000000;
constexpr uint32_t kJobManagerBase = 0x00000000;
constexpr uint32_t kMmuBase = 0x00002000;

constexpr uint32_t kGpuIdReg = kGpuControlBase + 0x000;
constexpr uint32_t kGpuVersionReg = kGpuControlBase + 0x004;
constexpr uint32_t kGpuStatusReg = kGpuControlBase + 0x008;
constexpr uint32_t kGpuIrqRawstatReg = kGpuControlBase + 0x020;
constexpr uint32_t kGpuIrqClearReg = kGpuControlBase + 0x024;
constexpr uint32_t kGpuIrqMaskReg = kGpuControlBase + 0x028;
constexpr uint32_t kGpuCmdReg = kGpuControlBase + 0x030;
constexpr uint32_t kGpuPwrKeyReg = kGpuControlBase + 0x050;
constexpr uint32_t kGpuPwrOverrideReg = kGpuControlBase + 0x054;

constexpr uint32_t kJobIrqRawstatReg = kJobManagerBase + 0x1000;
constexpr uint32_t kJobIrqClearReg = kJobManagerBase + 0x1004;
constexpr uint32_t kJobIrqMaskReg = kJobManagerBase + 0x1008;
constexpr uint32_t kJobControlReg = kJobManagerBase + 0x1010;

constexpr uint32_t kAsCommandReg = kMmuBase + 0x000;
constexpr uint32_t kAsStatusReg = kMmuBase + 0x004;
constexpr uint32_t kAsFaultstatus = kMmuBase + 0x008;
constexpr uint32_t kAsFaultaddressLo = kMmuBase + 0x00C;
constexpr uint32_t kAsFaultaddressHi = kMmuBase + 0x010;
constexpr uint32_t kAsTranstabLo = kMmuBase + 0x014;
constexpr uint32_t kAsTranstabHi = kMmuBase + 0x018;
constexpr uint32_t kAsMemattr = kMmuBase + 0x01C;

constexpr uint32_t kGpuCmdSoftReset = 0x01;
constexpr uint32_t kGpuCmdHardReset = 0x02;
constexpr uint32_t kGpuCmdPwrUp = 0x04;
constexpr uint32_t kGpuCmdPwrDown = 0x08;

constexpr uint32_t kGpuStatusActive = 0x01;
constexpr uint32_t kGpuStatusIdle = 0x02;
constexpr uint32_t kGpuStatusPwrActive = 0x04;

constexpr uint32_t kGpuIrqGpuFault = (1 << 0);
constexpr uint32_t kGpuIrqMmuFault = (1 << 2);
constexpr uint32_t kGpuIrqJobFinished = (1 << 4);
constexpr uint32_t kGpuIrqCacheClean = (1 << 5);

constexpr uint32_t kMaliG57ProductId = 0x9093;
constexpr uint32_t kValhallArchVersion = 0x0A;

}  // namespace mali_g57

#endif  // DRIVERS_GPU_MALI_G57_REGISTERS_H_
