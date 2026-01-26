

#include "../aic8800.h"

#include <fuchsia/hardware/sdio/cpp/banjo-mock.h>
#include <lib/ddk/device.h>
#include <lib/fake-bti/bti.h>
#include <lib/fake-resource/resource.h>
#include <zxtest/zxtest.h>

namespace aic8800 {
namespace {

class Aic8800InitTest : public zxtest::Test {
protected:
  void SetUp() override {
    fake_root_ = MockDevice::FakeRootParent();

    zx::bti out_bti;
    ASSERT_OK(fake_bti_create(out_bti.reset_and_get_address()));
  }

  void TearDown() override {}

  std::shared_ptr<MockDevice> fake_root_;
  ddk::MockSdioProtocolClient mock_sdio_;
};

TEST_F(Aic8800InitTest, DriverCreation) {
  auto device = new Aic8800(fake_root_.get());
  ASSERT_NOT_NULL(device);

  device->DdkRelease();
}

TEST_F(Aic8800InitTest, BindSuccess) {
  auto device = std::make_unique<Aic8800>(fake_root_.get());
  ASSERT_NOT_NULL(device.get());
}

TEST_F(Aic8800InitTest, SdioClientInitialization) {
  auto device = new Aic8800(fake_root_.get());
  ASSERT_NOT_NULL(device);

  device->DdkRelease();
}

TEST_F(Aic8800InitTest, InitializationSequence) {
  mock_sdio_.ExpectDoRwByte(true, 0, 0, 0).Return(ZX_OK, 0);
  mock_sdio_.ExpectDoRwByte(true, 0, 0, 0).Return(ZX_OK, 0);

  auto proto = mock_sdio_.GetProto();
  fake_root_->AddProtocol(ZX_PROTOCOL_SDIO, proto->ops, proto->ctx);

  auto device = std::make_unique<Aic8800>(fake_root_.get());
  ASSERT_NOT_NULL(device.get());
}

TEST_F(Aic8800InitTest, WlanphyQueryNotSupported) {
  auto device = new Aic8800(fake_root_.get());

  wlanphy_info_t info;
  zx_status_t status = device->WlanphyImplQuery(&info);
  EXPECT_EQ(status, ZX_ERR_NOT_SUPPORTED);

  device->DdkRelease();
}

TEST_F(Aic8800InitTest, CreateIfaceNotSupported) {
  auto device = new Aic8800(fake_root_.get());

  wlanphy_create_iface_req_t req = {};
  uint16_t iface_id = 0;
  zx_status_t status = device->WlanphyImplCreateIface(&req, &iface_id);
  EXPECT_EQ(status, ZX_ERR_NOT_SUPPORTED);

  device->DdkRelease();
}

TEST_F(Aic8800InitTest, DestroyIfaceNotSupported) {
  auto device = new Aic8800(fake_root_.get());

  zx_status_t status = device->WlanphyImplDestroyIface(0);
  EXPECT_EQ(status, ZX_ERR_NOT_SUPPORTED);

  device->DdkRelease();
}

TEST_F(Aic8800InitTest, SetCountryNotSupported) {
  auto device = new Aic8800(fake_root_.get());

  wlanphy_country_t country = {};
  zx_status_t status = device->WlanphyImplSetCountry(&country);
  EXPECT_EQ(status, ZX_ERR_NOT_SUPPORTED);

  device->DdkRelease();
}

TEST_F(Aic8800InitTest, ClearCountryNotSupported) {
  auto device = new Aic8800(fake_root_.get());

  zx_status_t status = device->WlanphyImplClearCountry();
  EXPECT_EQ(status, ZX_ERR_NOT_SUPPORTED);

  device->DdkRelease();
}

TEST_F(Aic8800InitTest, GetCountryNotSupported) {
  auto device = new Aic8800(fake_root_.get());

  wlanphy_country_t country;
  zx_status_t status = device->WlanphyImplGetCountry(&country);
  EXPECT_EQ(status, ZX_ERR_NOT_SUPPORTED);

  device->DdkRelease();
}

} // namespace
} // namespace aic8800
