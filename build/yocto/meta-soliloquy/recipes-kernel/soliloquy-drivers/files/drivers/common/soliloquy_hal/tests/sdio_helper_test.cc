#include "../sdio.h"

#include <fuchsia/hardware/sdio/cpp/banjo-mock.h>
#include <zxtest/zxtest.h>

namespace soliloquy_hal {
namespace {

class SdioHelperTest : public zxtest::Test {
protected:
  void SetUp() override {
    mock_sdio_ = ddk::MockSdioProtocolClient();
    helper_ = std::make_unique<SdioHelper>(&mock_sdio_);
  }

  ddk::MockSdioProtocolClient mock_sdio_;
  std::unique_ptr<SdioHelper> helper_;
};

TEST_F(SdioHelperTest, ReadByteSuccess) {
  constexpr uint32_t kAddress = 0x1000;
  constexpr uint8_t kExpectedValue = 0x42;
  
  uint8_t out_byte = 0;
  mock_sdio_.ExpectDoRwByte(false, kAddress, 0, &out_byte)
      .Return(ZX_OK, kExpectedValue);
  
  uint8_t result = 0;
  zx_status_t status = helper_->ReadByte(kAddress, &result);
  
  EXPECT_OK(status);
  EXPECT_EQ(result, kExpectedValue);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, ReadByteNullPointer) {
  zx_status_t status = helper_->ReadByte(0x1000, nullptr);
  EXPECT_EQ(status, ZX_ERR_INVALID_ARGS);
}

TEST_F(SdioHelperTest, ReadByteFailure) {
  constexpr uint32_t kAddress = 0x2000;
  uint8_t dummy = 0;
  
  mock_sdio_.ExpectDoRwByte(false, kAddress, 0, &dummy)
      .Return(ZX_ERR_IO, 0);
  
  uint8_t result = 0;
  zx_status_t status = helper_->ReadByte(kAddress, &result);
  
  EXPECT_EQ(status, ZX_ERR_IO);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, WriteByteSuccess) {
  constexpr uint32_t kAddress = 0x3000;
  constexpr uint8_t kValue = 0xAB;
  
  mock_sdio_.ExpectDoRwByte(true, kAddress, kValue, nullptr)
      .Return(ZX_OK, 0);
  
  zx_status_t status = helper_->WriteByte(kAddress, kValue);
  
  EXPECT_OK(status);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, WriteByteFailure) {
  constexpr uint32_t kAddress = 0x4000;
  constexpr uint8_t kValue = 0xCD;
  
  mock_sdio_.ExpectDoRwByte(true, kAddress, kValue, nullptr)
      .Return(ZX_ERR_TIMED_OUT, 0);
  
  zx_status_t status = helper_->WriteByte(kAddress, kValue);
  
  EXPECT_EQ(status, ZX_ERR_TIMED_OUT);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, ReadMultiBlockNullBuffer) {
  uint8_t* null_buf = nullptr;
  zx_status_t status = helper_->ReadMultiBlock(0x5000, null_buf, 100);
  EXPECT_EQ(status, ZX_ERR_INVALID_ARGS);
}

TEST_F(SdioHelperTest, ReadMultiBlockZeroLength) {
  uint8_t buffer[10];
  zx_status_t status = helper_->ReadMultiBlock(0x5000, buffer, 0);
  EXPECT_EQ(status, ZX_ERR_INVALID_ARGS);
}

TEST_F(SdioHelperTest, ReadMultiBlockSingleBlock) {
  constexpr uint32_t kAddress = 0x6000;
  constexpr size_t kLength = 256;
  uint8_t buffer[kLength];
  
  mock_sdio_.ExpectDoRwTxn(kAddress, buffer, kLength, false, false)
      .Return(ZX_OK);
  
  zx_status_t status = helper_->ReadMultiBlock(kAddress, buffer, kLength);
  
  EXPECT_OK(status);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, ReadMultiBlockMultipleBlocks) {
  constexpr uint32_t kAddress = 0x7000;
  constexpr size_t kBlockSize = 512;
  constexpr size_t kLength = 1024;
  uint8_t buffer[kLength];
  
  mock_sdio_.ExpectDoRwTxn(kAddress, buffer, kBlockSize, false, false)
      .Return(ZX_OK);
  mock_sdio_.ExpectDoRwTxn(kAddress + kBlockSize, buffer + kBlockSize, 
                           kBlockSize, false, false)
      .Return(ZX_OK);
  
  zx_status_t status = helper_->ReadMultiBlock(kAddress, buffer, kLength);
  
  EXPECT_OK(status);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, ReadMultiBlockPartialBlock) {
  constexpr uint32_t kAddress = 0x8000;
  constexpr size_t kLength = 300;
  uint8_t buffer[kLength];
  
  mock_sdio_.ExpectDoRwTxn(kAddress, buffer, kLength, false, false)
      .Return(ZX_OK);
  
  zx_status_t status = helper_->ReadMultiBlock(kAddress, buffer, kLength);
  
  EXPECT_OK(status);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, ReadMultiBlockFailureFirstBlock) {
  constexpr uint32_t kAddress = 0x9000;
  constexpr size_t kLength = 1024;
  uint8_t buffer[kLength];
  
  mock_sdio_.ExpectDoRwTxn(kAddress, buffer, 512, false, false)
      .Return(ZX_ERR_IO);
  
  zx_status_t status = helper_->ReadMultiBlock(kAddress, buffer, kLength);
  
  EXPECT_EQ(status, ZX_ERR_IO);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, ReadMultiBlockFailureSecondBlock) {
  constexpr uint32_t kAddress = 0xA000;
  constexpr size_t kBlockSize = 512;
  constexpr size_t kLength = 1024;
  uint8_t buffer[kLength];
  
  mock_sdio_.ExpectDoRwTxn(kAddress, buffer, kBlockSize, false, false)
      .Return(ZX_OK);
  mock_sdio_.ExpectDoRwTxn(kAddress + kBlockSize, buffer + kBlockSize, 
                           kBlockSize, false, false)
      .Return(ZX_ERR_INTERNAL);
  
  zx_status_t status = helper_->ReadMultiBlock(kAddress, buffer, kLength);
  
  EXPECT_EQ(status, ZX_ERR_INTERNAL);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, WriteMultiBlockNullBuffer) {
  const uint8_t* null_buf = nullptr;
  zx_status_t status = helper_->WriteMultiBlock(0xB000, null_buf, 100);
  EXPECT_EQ(status, ZX_ERR_INVALID_ARGS);
}

TEST_F(SdioHelperTest, WriteMultiBlockZeroLength) {
  uint8_t buffer[10] = {0};
  zx_status_t status = helper_->WriteMultiBlock(0xB000, buffer, 0);
  EXPECT_EQ(status, ZX_ERR_INVALID_ARGS);
}

TEST_F(SdioHelperTest, WriteMultiBlockSingleBlock) {
  constexpr uint32_t kAddress = 0xC000;
  constexpr size_t kLength = 256;
  uint8_t buffer[kLength] = {0};
  
  mock_sdio_.ExpectDoRwTxn(kAddress, const_cast<uint8_t*>(buffer), 
                           kLength, true, false)
      .Return(ZX_OK);
  
  zx_status_t status = helper_->WriteMultiBlock(kAddress, buffer, kLength);
  
  EXPECT_OK(status);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, WriteMultiBlockMultipleBlocks) {
  constexpr uint32_t kAddress = 0xD000;
  constexpr size_t kBlockSize = 512;
  constexpr size_t kLength = 1024;
  uint8_t buffer[kLength] = {0};
  
  for (size_t i = 0; i < kLength; i++) {
    buffer[i] = static_cast<uint8_t>(i & 0xFF);
  }
  
  mock_sdio_.ExpectDoRwTxn(kAddress, const_cast<uint8_t*>(buffer), 
                           kBlockSize, true, false)
      .Return(ZX_OK);
  mock_sdio_.ExpectDoRwTxn(kAddress + kBlockSize, 
                           const_cast<uint8_t*>(buffer + kBlockSize), 
                           kBlockSize, true, false)
      .Return(ZX_OK);
  
  zx_status_t status = helper_->WriteMultiBlock(kAddress, buffer, kLength);
  
  EXPECT_OK(status);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, WriteMultiBlockFailurePropagation) {
  constexpr uint32_t kAddress = 0xE000;
  constexpr size_t kBlockSize = 512;
  constexpr size_t kLength = 1536;
  uint8_t buffer[kLength] = {0};
  
  mock_sdio_.ExpectDoRwTxn(kAddress, const_cast<uint8_t*>(buffer), 
                           kBlockSize, true, false)
      .Return(ZX_OK);
  mock_sdio_.ExpectDoRwTxn(kAddress + kBlockSize, 
                           const_cast<uint8_t*>(buffer + kBlockSize), 
                           kBlockSize, true, false)
      .Return(ZX_ERR_NOT_SUPPORTED);
  
  zx_status_t status = helper_->WriteMultiBlock(kAddress, buffer, kLength);
  
  EXPECT_EQ(status, ZX_ERR_NOT_SUPPORTED);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, WriteMultiBlockExactBlockBoundary) {
  constexpr uint32_t kAddress = 0xF000;
  constexpr size_t kLength = 512;
  uint8_t buffer[kLength] = {0};
  
  mock_sdio_.ExpectDoRwTxn(kAddress, const_cast<uint8_t*>(buffer), 
                           kLength, true, false)
      .Return(ZX_OK);
  
  zx_status_t status = helper_->WriteMultiBlock(kAddress, buffer, kLength);
  
  EXPECT_OK(status);
  mock_sdio_.VerifyAndClear();
}

TEST_F(SdioHelperTest, WriteMultiBlockWiFiDriverPattern) {
  constexpr uint32_t kRegAddr = 0x00100000;
  constexpr size_t kDataLength = 2048;
  uint8_t tx_buffer[kDataLength];
  
  for (size_t i = 0; i < kDataLength; i++) {
    tx_buffer[i] = static_cast<uint8_t>((i * 7) & 0xFF);
  }
  
  mock_sdio_.ExpectDoRwTxn(kRegAddr, const_cast<uint8_t*>(tx_buffer), 
                           512, true, false).Return(ZX_OK);
  mock_sdio_.ExpectDoRwTxn(kRegAddr + 512, const_cast<uint8_t*>(tx_buffer + 512), 
                           512, true, false).Return(ZX_OK);
  mock_sdio_.ExpectDoRwTxn(kRegAddr + 1024, const_cast<uint8_t*>(tx_buffer + 1024), 
                           512, true, false).Return(ZX_OK);
  mock_sdio_.ExpectDoRwTxn(kRegAddr + 1536, const_cast<uint8_t*>(tx_buffer + 1536), 
                           512, true, false).Return(ZX_OK);
  
  zx_status_t status = helper_->WriteMultiBlock(kRegAddr, tx_buffer, kDataLength);
  EXPECT_OK(status);
  mock_sdio_.VerifyAndClear();
}

} // namespace
} // namespace soliloquy_hal
