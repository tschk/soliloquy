

#include "../mmio.h"

#include <lib/fake-mmio-reg/fake-mmio-reg.h>
#include <lib/mmio/mmio.h>
#include <zxtest/zxtest.h>

namespace soliloquy_hal {
namespace {

constexpr size_t kRegisterCount = 32;
constexpr size_t kRegisterSize = sizeof(uint32_t);

class MmioHelperTest : public zxtest::Test {
protected:
  void SetUp() override {
    fake_mmio_regs_ = std::make_unique<ddk_fake::FakeMmioReg[]>(kRegisterCount);
    fake_mmio_ = std::make_unique<ddk_fake::FakeMmioRegRegion>(
        fake_mmio_regs_.get(), kRegisterSize, kRegisterCount);
    mmio_buffer_ = fake_mmio_->GetMmioBuffer();
    helper_ = std::make_unique<MmioHelper>(&mmio_buffer_);
  }

  std::unique_ptr<ddk_fake::FakeMmioReg[]> fake_mmio_regs_;
  std::unique_ptr<ddk_fake::FakeMmioRegRegion> fake_mmio_;
  ddk::MmioBuffer mmio_buffer_;
  std::unique_ptr<MmioHelper> helper_;
};

TEST_F(MmioHelperTest, Read32) {
  constexpr uint32_t kTestValue = 0x12345678;
  fake_mmio_regs_[0].SetReadCallback([&]() { return kTestValue; });

  uint32_t value = helper_->Read32(0);
  EXPECT_EQ(value, kTestValue);
}

TEST_F(MmioHelperTest, Write32) {
  constexpr uint32_t kTestValue = 0xABCDEF00;
  bool write_called = false;

  fake_mmio_regs_[0].SetWriteCallback([&](uint64_t value) {
    write_called = true;
    EXPECT_EQ(value, kTestValue);
  });

  helper_->Write32(0, kTestValue);
  EXPECT_TRUE(write_called);
}

TEST_F(MmioHelperTest, SetBits32) {
  constexpr uint32_t kInitialValue = 0x00000000;
  constexpr uint32_t kMask = 0x0000FF00;
  constexpr uint32_t kExpectedValue = 0x0000FF00;

  fake_mmio_regs_[0].SetReadCallback([&]() { return kInitialValue; });

  uint32_t written_value = 0;
  fake_mmio_regs_[0].SetWriteCallback(
      [&](uint64_t value) { written_value = static_cast<uint32_t>(value); });

  helper_->SetBits32(0, kMask);
  EXPECT_EQ(written_value, kExpectedValue);
}

TEST_F(MmioHelperTest, ClearBits32) {
  constexpr uint32_t kInitialValue = 0xFFFFFFFF;
  constexpr uint32_t kMask = 0x0000FF00;
  constexpr uint32_t kExpectedValue = 0xFFFF00FF;

  fake_mmio_regs_[0].SetReadCallback([&]() { return kInitialValue; });

  uint32_t written_value = 0;
  fake_mmio_regs_[0].SetWriteCallback(
      [&](uint64_t value) { written_value = static_cast<uint32_t>(value); });

  helper_->ClearBits32(0, kMask);
  EXPECT_EQ(written_value, kExpectedValue);
}

TEST_F(MmioHelperTest, ModifyBits32) {
  constexpr uint32_t kInitialValue = 0x12345678;
  constexpr uint32_t kMask = 0x0000FF00;
  constexpr uint32_t kNewValue = 0x0000AB00;
  constexpr uint32_t kExpectedValue = 0x1234AB78;

  fake_mmio_regs_[0].SetReadCallback([&]() { return kInitialValue; });

  uint32_t written_value = 0;
  fake_mmio_regs_[0].SetWriteCallback(
      [&](uint64_t value) { written_value = static_cast<uint32_t>(value); });

  helper_->ModifyBits32(0, kMask, kNewValue);
  EXPECT_EQ(written_value, kExpectedValue);
}

TEST_F(MmioHelperTest, ReadMasked32) {
  constexpr uint32_t kRegisterValue = 0x12345678;
  constexpr uint32_t kMask = 0x0000FF00;
  constexpr uint32_t kShift = 8;
  constexpr uint32_t kExpectedValue = 0x56;

  fake_mmio_regs_[0].SetReadCallback([&]() { return kRegisterValue; });

  uint32_t value = helper_->ReadMasked32(0, kMask, kShift);
  EXPECT_EQ(value, kExpectedValue);
}

TEST_F(MmioHelperTest, WriteMasked32) {
  constexpr uint32_t kInitialValue = 0x12345678;
  constexpr uint32_t kMask = 0x0000FF00;
  constexpr uint32_t kShift = 8;
  constexpr uint32_t kNewValue = 0xAB;
  constexpr uint32_t kExpectedValue = 0x1234AB78;

  fake_mmio_regs_[0].SetReadCallback([&]() { return kInitialValue; });

  uint32_t written_value = 0;
  fake_mmio_regs_[0].SetWriteCallback(
      [&](uint64_t value) { written_value = static_cast<uint32_t>(value); });

  helper_->WriteMasked32(0, kMask, kShift, kNewValue);
  EXPECT_EQ(written_value, kExpectedValue);
}

TEST_F(MmioHelperTest, WaitForBit32Success) {
  constexpr uint32_t kBit = 5;
  constexpr uint32_t kMask = 1 << kBit;

  size_t read_count = 0;
  fake_mmio_regs_[0].SetReadCallback([&]() {
    read_count++;
    if (read_count >= 3) {
      return kMask;
    }
    return 0u;
  });

  bool result = helper_->WaitForBit32(0, kBit, true, zx::msec(100));
  EXPECT_TRUE(result);
  EXPECT_GE(read_count, 3u);
}

TEST_F(MmioHelperTest, WaitForBit32Timeout) {
  constexpr uint32_t kBit = 7;

  fake_mmio_regs_[0].SetReadCallback([&]() { return 0u; });

  bool result = helper_->WaitForBit32(0, kBit, true, zx::msec(10));
  EXPECT_FALSE(result);
}

TEST_F(MmioHelperTest, WaitForBit32ClearSuccess) {
  constexpr uint32_t kBit = 3;
  constexpr uint32_t kMask = 1 << kBit;

  size_t read_count = 0;
  fake_mmio_regs_[0].SetReadCallback([&]() {
    read_count++;
    if (read_count >= 2) {
      return 0u;
    }
    return kMask;
  });

  bool result = helper_->WaitForBit32(0, kBit, false, zx::msec(100));
  EXPECT_TRUE(result);
  EXPECT_GE(read_count, 2u);
}

} // namespace
} // namespace soliloquy_hal
