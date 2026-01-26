pub struct Aic8800Registers;

impl Aic8800Registers {
    pub const VENDOR_ID: u32 = 0xA5C8;
    pub const DEVICE_ID: u32 = 0x8800;
    
    pub const SDIO_FUNC0_CCCR: u32 = 0x00000000;
    pub const SDIO_FUNC1_FBR: u32 = 0x00000100;
    
    pub const REG_CHIP_ID: u32 = 0x00000000;
    pub const REG_CHIP_REV: u32 = 0x00000004;
    pub const REG_FW_STATUS: u32 = 0x00000008;
    pub const REG_HOST_CTRL: u32 = 0x0000000C;
    pub const REG_INT_STATUS: u32 = 0x00000010;
    pub const REG_INT_MASK: u32 = 0x00000014;
    pub const REG_TX_READY: u32 = 0x00000018;
    pub const REG_RX_READY: u32 = 0x0000001C;
    
    pub const REG_SDIO_CTRL: u32 = 0x00000100;
    pub const REG_BLOCK_SIZE: u32 = 0x00000110;
    pub const REG_BLOCK_COUNT: u32 = 0x00000114;
    
    pub const REG_FW_DOWNLOAD_ADDR: u32 = 0x00100000;
    pub const REG_FW_DOWNLOAD_SIZE: u32 = 0x00100004;
    pub const REG_FW_DOWNLOAD_CTRL: u32 = 0x00100008;
    
    pub const REG_MAC_ADDR_LOW: u32 = 0x00001000;
    pub const REG_MAC_ADDR_HIGH: u32 = 0x00001004;
    
    pub const REG_PHY_CTRL: u32 = 0x00002000;
    pub const REG_RF_CTRL: u32 = 0x00002004;
    pub const REG_AGC_CTRL: u32 = 0x00002008;
    
    pub const INT_FW_READY: u32 = 1 << 0;
    pub const INT_TX_DONE: u32 = 1 << 1;
    pub const INT_RX_READY: u32 = 1 << 2;
    pub const INT_ERROR: u32 = 1 << 31;
    
    pub const HOST_CTRL_RESET: u32 = 1 << 0;
    pub const HOST_CTRL_ENABLE: u32 = 1 << 1;
    pub const HOST_CTRL_SLEEP: u32 = 1 << 2;
    
    pub const FW_STATUS_IDLE: u32 = 0;
    pub const FW_STATUS_DOWNLOADING: u32 = 1;
    pub const FW_STATUS_READY: u32 = 2;
    pub const FW_STATUS_ERROR: u32 = 0xFF;
    
    pub const FW_DOWNLOAD_START: u32 = 1 << 0;
    pub const FW_DOWNLOAD_DONE: u32 = 1 << 1;
    pub const FW_DOWNLOAD_ERROR: u32 = 1 << 31;
    
    pub const BLOCK_SIZE_DEFAULT: usize = 512;
    pub const BLOCK_SIZE_MAX: usize = 2048;
    
    pub const FW_BASE_ADDR: u32 = 0x00100000;
    pub const FW_MAX_SIZE: usize = 512 * 1024;
    
    pub const TIMEOUT_MS_SHORT: u64 = 100;
    pub const TIMEOUT_MS_MEDIUM: u64 = 1000;
    pub const TIMEOUT_MS_LONG: u64 = 5000;
    
    pub fn chip_id_to_string(chip_id: u32) -> &'static str {
        match chip_id {
            0x88000000 => "AIC8800D",
            0x88000001 => "AIC8800DC",
            0x88000002 => "AIC8800DW",
            _ => "Unknown",
        }
    }
    
    pub fn is_valid_chip_id(chip_id: u32) -> bool {
        matches!(chip_id, 0x88000000..=0x88000002)
    }
    
    pub fn is_fw_ready(fw_status: u32) -> bool {
        fw_status == Self::FW_STATUS_READY
    }
    
    pub fn has_error(status: u32) -> bool {
        (status & Self::INT_ERROR) != 0
    }
}

pub struct Aic8800RegisterMap {
    registers: std::collections::HashMap<u32, u32>,
}

impl Aic8800RegisterMap {
    pub fn new() -> Self {
        let mut map = Self {
            registers: std::collections::HashMap::new(),
        };
        
        map.write(Aic8800Registers::REG_CHIP_ID, 0x88000001);
        map.write(Aic8800Registers::REG_CHIP_REV, 0x00000002);
        map.write(Aic8800Registers::REG_FW_STATUS, Aic8800Registers::FW_STATUS_IDLE);
        map.write(Aic8800Registers::REG_BLOCK_SIZE, Aic8800Registers::BLOCK_SIZE_DEFAULT as u32);
        
        map
    }
    
    pub fn read(&self, address: u32) -> u32 {
        self.registers.get(&address).copied().unwrap_or(0)
    }
    
    pub fn write(&mut self, address: u32, value: u32) {
        self.registers.insert(address, value);
    }
    
    pub fn set_bits(&mut self, address: u32, mask: u32) {
        let current = self.read(address);
        self.write(address, current | mask);
    }
    
    pub fn clear_bits(&mut self, address: u32, mask: u32) {
        let current = self.read(address);
        self.write(address, current & !mask);
    }
    
    pub fn modify_bits(&mut self, address: u32, mask: u32, value: u32) {
        let current = self.read(address);
        self.write(address, (current & !mask) | (value & mask));
    }
}

impl Default for Aic8800RegisterMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chip_id_validation() {
        assert!(Aic8800Registers::is_valid_chip_id(0x88000000));
        assert!(Aic8800Registers::is_valid_chip_id(0x88000001));
        assert!(Aic8800Registers::is_valid_chip_id(0x88000002));
        assert!(!Aic8800Registers::is_valid_chip_id(0x12345678));
    }

    #[test]
    fn test_chip_id_to_string() {
        assert_eq!(Aic8800Registers::chip_id_to_string(0x88000000), "AIC8800D");
        assert_eq!(Aic8800Registers::chip_id_to_string(0x88000001), "AIC8800DC");
        assert_eq!(Aic8800Registers::chip_id_to_string(0x88000002), "AIC8800DW");
        assert_eq!(Aic8800Registers::chip_id_to_string(0x12345678), "Unknown");
    }

    #[test]
    fn test_register_map() {
        let mut map = Aic8800RegisterMap::new();
        
        assert_eq!(map.read(Aic8800Registers::REG_CHIP_ID), 0x88000001);
        
        map.write(Aic8800Registers::REG_HOST_CTRL, 0x12345678);
        assert_eq!(map.read(Aic8800Registers::REG_HOST_CTRL), 0x12345678);
    }

    #[test]
    fn test_bit_operations() {
        let mut map = Aic8800RegisterMap::new();
        
        map.write(Aic8800Registers::REG_INT_MASK, 0x00000000);
        map.set_bits(Aic8800Registers::REG_INT_MASK, 0x0F);
        assert_eq!(map.read(Aic8800Registers::REG_INT_MASK), 0x0F);
        
        map.clear_bits(Aic8800Registers::REG_INT_MASK, 0x03);
        assert_eq!(map.read(Aic8800Registers::REG_INT_MASK), 0x0C);
        
        map.modify_bits(Aic8800Registers::REG_INT_MASK, 0xFF, 0x42);
        assert_eq!(map.read(Aic8800Registers::REG_INT_MASK), 0x42);
    }

    #[test]
    fn test_fw_status_checks() {
        assert!(Aic8800Registers::is_fw_ready(Aic8800Registers::FW_STATUS_READY));
        assert!(!Aic8800Registers::is_fw_ready(Aic8800Registers::FW_STATUS_IDLE));
    }

    #[test]
    fn test_error_detection() {
        assert!(Aic8800Registers::has_error(Aic8800Registers::INT_ERROR));
        assert!(Aic8800Registers::has_error(Aic8800Registers::INT_ERROR | Aic8800Registers::INT_TX_DONE));
        assert!(!Aic8800Registers::has_error(Aic8800Registers::INT_TX_DONE));
    }
}
