use aic8800_mock::{MockSdioDevice, MockFirmwareLoader, Aic8800Registers};

#[test]
fn test_full_initialization_flow() {
    env_logger::init();
    
    let sdio = MockSdioDevice::new();
    let fw_loader = MockFirmwareLoader::new();
    
    assert!(sdio.initialize().is_ok());
    assert!(sdio.set_block_size(512).is_ok());
    
    let firmware = MockFirmwareLoader::create_aic8800_firmware();
    fw_loader.add_firmware("fmacfw_8800d80.bin", firmware.clone());
    
    let loaded_fw = fw_loader.load_firmware("fmacfw_8800d80.bin").unwrap();
    assert_eq!(loaded_fw.len(), firmware.len());
    
    let result = sdio.download_firmware(Aic8800Registers::FW_BASE_ADDR, &loaded_fw);
    assert!(result.is_ok());
    
    assert!(sdio.verify_firmware_at(Aic8800Registers::FW_BASE_ADDR, &loaded_fw));
}

#[test]
fn test_register_initialization_sequence() {
    let sdio = MockSdioDevice::new();
    sdio.initialize().unwrap();
    sdio.set_block_size(Aic8800Registers::BLOCK_SIZE_DEFAULT).unwrap();
    
    sdio.write_byte(Aic8800Registers::REG_HOST_CTRL, 
                    (Aic8800Registers::HOST_CTRL_RESET) as u8).unwrap();
    
    let ctrl_val = sdio.read_byte(Aic8800Registers::REG_HOST_CTRL).unwrap();
    assert_eq!(ctrl_val, Aic8800Registers::HOST_CTRL_RESET as u8);
    
    sdio.write_byte(Aic8800Registers::REG_HOST_CTRL, 
                    (Aic8800Registers::HOST_CTRL_ENABLE) as u8).unwrap();
    
    let ctrl_val = sdio.read_byte(Aic8800Registers::REG_HOST_CTRL).unwrap();
    assert_eq!(ctrl_val, Aic8800Registers::HOST_CTRL_ENABLE as u8);
}

#[test]
fn test_firmware_download_with_verification() {
    let sdio = MockSdioDevice::new();
    let fw_loader = MockFirmwareLoader::new();
    
    sdio.initialize().unwrap();
    sdio.set_block_size(512).unwrap();
    
    let firmware = vec![0xAA; 2048];
    fw_loader.add_firmware("test_fw.bin", firmware.clone());
    
    let loaded_fw = fw_loader.load_firmware("test_fw.bin").unwrap();
    sdio.download_firmware(0x00100000, &loaded_fw).unwrap();
    
    assert!(sdio.verify_firmware_at(0x00100000, &loaded_fw));
    
    let read_back = sdio.read_multi_block(0x00100000, firmware.len()).unwrap();
    assert_eq!(read_back, firmware);
}

#[test]
fn test_error_recovery() {
    let sdio = MockSdioDevice::new();
    sdio.initialize().unwrap();
    
    sdio.write_byte(0x1000, 0x42).unwrap();
    
    sdio.fail_next_operation();
    let result = sdio.read_byte(0x1000);
    assert!(result.is_err());
    
    let value = sdio.read_byte(0x1000).unwrap();
    assert_eq!(value, 0x42);
}

#[test]
fn test_transaction_history() {
    let sdio = MockSdioDevice::new();
    sdio.initialize().unwrap();
    
    sdio.clear_transactions();
    
    sdio.write_byte(0x1000, 0x11).unwrap();
    sdio.write_byte(0x1004, 0x22).unwrap();
    sdio.read_byte(0x1000).unwrap();
    
    let transactions = sdio.get_transactions();
    assert_eq!(transactions.len(), 3);
    
    assert!(transactions[0].is_write);
    assert_eq!(transactions[0].address, 0x1000);
    assert_eq!(transactions[0].data[0], 0x11);
    
    assert!(transactions[1].is_write);
    assert_eq!(transactions[1].address, 0x1004);
    assert_eq!(transactions[1].data[0], 0x22);
    
    assert!(!transactions[2].is_write);
    assert_eq!(transactions[2].address, 0x1000);
}

#[test]
fn test_block_transfer() {
    let sdio = MockSdioDevice::new();
    sdio.initialize().unwrap();
    sdio.set_block_size(512).unwrap();
    
    let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
    
    sdio.write_multi_block(0x2000, &data).unwrap();
    
    let read_data = sdio.read_multi_block(0x2000, data.len()).unwrap();
    assert_eq!(read_data, data);
}

#[test]
fn test_interrupt_status_handling() {
    let sdio = MockSdioDevice::new();
    sdio.initialize().unwrap();
    
    let status = Aic8800Registers::INT_FW_READY | Aic8800Registers::INT_TX_DONE;
    let status_bytes = status.to_le_bytes();
    for (i, &byte) in status_bytes.iter().enumerate() {
        sdio.write_byte(Aic8800Registers::REG_INT_STATUS + i as u32, byte).unwrap();
    }
    
    let mut read_status_bytes = [0u8; 4];
    for i in 0..4 {
        read_status_bytes[i] = sdio.read_byte(Aic8800Registers::REG_INT_STATUS + i as u32).unwrap();
    }
    let read_status = u32::from_le_bytes(read_status_bytes);
    assert_eq!(read_status, status);
    
    assert!(!Aic8800Registers::has_error(read_status));
    
    let error_status_val = status | Aic8800Registers::INT_ERROR;
    let error_status_bytes = error_status_val.to_le_bytes();
    for (i, &byte) in error_status_bytes.iter().enumerate() {
        sdio.write_byte(Aic8800Registers::REG_INT_STATUS + i as u32, byte).unwrap();
    }
    
    let mut error_status_bytes_read = [0u8; 4];
    for i in 0..4 {
        error_status_bytes_read[i] = sdio.read_byte(Aic8800Registers::REG_INT_STATUS + i as u32).unwrap();
    }
    let error_status = u32::from_le_bytes(error_status_bytes_read);
    assert!(Aic8800Registers::has_error(error_status));
}

#[test]
fn test_firmware_status_transitions() {
    let sdio = MockSdioDevice::new();
    sdio.initialize().unwrap();
    
    sdio.write_byte(Aic8800Registers::REG_FW_STATUS, 
                    Aic8800Registers::FW_STATUS_IDLE as u8).unwrap();
    let status = sdio.read_byte(Aic8800Registers::REG_FW_STATUS).unwrap() as u32;
    assert!(!Aic8800Registers::is_fw_ready(status));
    
    sdio.write_byte(Aic8800Registers::REG_FW_STATUS, 
                    Aic8800Registers::FW_STATUS_DOWNLOADING as u8).unwrap();
    let status = sdio.read_byte(Aic8800Registers::REG_FW_STATUS).unwrap() as u32;
    assert!(!Aic8800Registers::is_fw_ready(status));
    
    sdio.write_byte(Aic8800Registers::REG_FW_STATUS, 
                    Aic8800Registers::FW_STATUS_READY as u8).unwrap();
    let status = sdio.read_byte(Aic8800Registers::REG_FW_STATUS).unwrap() as u32;
    assert!(Aic8800Registers::is_fw_ready(status));
}

#[test]
fn test_mac_address_operations() {
    let sdio = MockSdioDevice::new();
    sdio.initialize().unwrap();
    
    let mac_low: u32 = 0x12345678;
    let mac_high: u32 = 0xABCD;
    
    for (i, &byte) in mac_low.to_le_bytes().iter().enumerate() {
        sdio.write_byte(Aic8800Registers::REG_MAC_ADDR_LOW + i as u32, byte).unwrap();
    }
    
    for (i, &byte) in mac_high.to_le_bytes().iter().enumerate() {
        sdio.write_byte(Aic8800Registers::REG_MAC_ADDR_HIGH + i as u32, byte).unwrap();
    }
    
    let mut read_mac_low = [0u8; 4];
    for i in 0..4 {
        read_mac_low[i] = sdio.read_byte(Aic8800Registers::REG_MAC_ADDR_LOW + i as u32).unwrap();
    }
    assert_eq!(u32::from_le_bytes(read_mac_low), mac_low);
}

#[test]
fn test_multiple_firmware_loads() {
    let fw_loader = MockFirmwareLoader::new();
    
    let fw1 = MockFirmwareLoader::create_test_firmware(1024, 0xAA);
    let fw2 = MockFirmwareLoader::create_test_firmware(2048, 0xBB);
    let fw3 = MockFirmwareLoader::create_aic8800_firmware();
    
    fw_loader.add_firmware("fw1.bin", fw1.clone());
    fw_loader.add_firmware("fw2.bin", fw2.clone());
    fw_loader.add_firmware("fw3.bin", fw3.clone());
    
    assert_eq!(fw_loader.load_firmware("fw1.bin").unwrap(), fw1);
    assert_eq!(fw_loader.load_firmware("fw2.bin").unwrap(), fw2);
    assert_eq!(fw_loader.load_firmware("fw3.bin").unwrap(), fw3);
    
    assert_eq!(fw_loader.get_load_count("fw1.bin"), 1);
    assert_eq!(fw_loader.get_load_count("fw2.bin"), 1);
    assert_eq!(fw_loader.get_load_count("fw3.bin"), 1);
}
