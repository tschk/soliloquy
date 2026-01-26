use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdioError {
    InvalidAddress,
    InvalidLength,
    NotInitialized,
    TransferError,
    Timeout,
}

pub type SdioResult<T> = Result<T, SdioError>;

#[derive(Debug, Clone)]
pub struct SdioTransaction {
    pub address: u32,
    pub data: Vec<u8>,
    pub is_write: bool,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct MockSdioDevice {
    memory: Arc<Mutex<HashMap<u32, u8>>>,
    transactions: Arc<Mutex<Vec<SdioTransaction>>>,
    block_size: Arc<Mutex<usize>>,
    initialized: Arc<Mutex<bool>>,
    fail_next: Arc<Mutex<bool>>,
    transaction_counter: Arc<Mutex<u64>>,
}

impl MockSdioDevice {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(Vec::new())),
            block_size: Arc::new(Mutex::new(512)),
            initialized: Arc::new(Mutex::new(false)),
            fail_next: Arc::new(Mutex::new(false)),
            transaction_counter: Arc::new(Mutex::new(0)),
        }
    }

    pub fn initialize(&self) -> SdioResult<()> {
        let mut init = self.initialized.lock().unwrap();
        *init = true;
        log::info!("MockSdioDevice initialized");
        Ok(())
    }

    pub fn set_block_size(&self, size: usize) -> SdioResult<()> {
        if !*self.initialized.lock().unwrap() {
            return Err(SdioError::NotInitialized);
        }
        let mut block_size = self.block_size.lock().unwrap();
        *block_size = size;
        log::info!("SDIO block size set to {}", size);
        Ok(())
    }

    pub fn get_block_size(&self) -> usize {
        *self.block_size.lock().unwrap()
    }

    pub fn read_byte(&self, address: u32) -> SdioResult<u8> {
        if !*self.initialized.lock().unwrap() {
            return Err(SdioError::NotInitialized);
        }

        if *self.fail_next.lock().unwrap() {
            *self.fail_next.lock().unwrap() = false;
            return Err(SdioError::TransferError);
        }

        let memory = self.memory.lock().unwrap();
        let value = memory.get(&address).copied().unwrap_or(0);
        
        self.record_transaction(address, vec![value], false);
        
        log::debug!("SDIO read byte: addr=0x{:08x}, value=0x{:02x}", address, value);
        Ok(value)
    }

    pub fn write_byte(&self, address: u32, value: u8) -> SdioResult<()> {
        if !*self.initialized.lock().unwrap() {
            return Err(SdioError::NotInitialized);
        }

        if *self.fail_next.lock().unwrap() {
            *self.fail_next.lock().unwrap() = false;
            return Err(SdioError::TransferError);
        }

        let mut memory = self.memory.lock().unwrap();
        memory.insert(address, value);
        
        self.record_transaction(address, vec![value], true);
        
        log::debug!("SDIO write byte: addr=0x{:08x}, value=0x{:02x}", address, value);
        Ok(())
    }

    pub fn read_multi_block(&self, address: u32, length: usize) -> SdioResult<Vec<u8>> {
        if !*self.initialized.lock().unwrap() {
            return Err(SdioError::NotInitialized);
        }

        if *self.fail_next.lock().unwrap() {
            *self.fail_next.lock().unwrap() = false;
            return Err(SdioError::TransferError);
        }

        let memory = self.memory.lock().unwrap();
        let mut data = Vec::with_capacity(length);
        
        for i in 0..length {
            let addr = address.wrapping_add(i as u32);
            let value = memory.get(&addr).copied().unwrap_or(0);
            data.push(value);
        }
        
        self.record_transaction(address, data.clone(), false);
        
        log::debug!("SDIO read multi-block: addr=0x{:08x}, len={}", address, length);
        Ok(data)
    }

    pub fn write_multi_block(&self, address: u32, data: &[u8]) -> SdioResult<()> {
        if !*self.initialized.lock().unwrap() {
            return Err(SdioError::NotInitialized);
        }

        if *self.fail_next.lock().unwrap() {
            *self.fail_next.lock().unwrap() = false;
            return Err(SdioError::TransferError);
        }

        let mut memory = self.memory.lock().unwrap();
        
        for (i, &byte) in data.iter().enumerate() {
            let addr = address.wrapping_add(i as u32);
            memory.insert(addr, byte);
        }
        
        self.record_transaction(address, data.to_vec(), true);
        
        log::debug!("SDIO write multi-block: addr=0x{:08x}, len={}", address, data.len());
        Ok(())
    }

    pub fn download_firmware(&self, base_address: u32, firmware_data: &[u8]) -> SdioResult<()> {
        if !*self.initialized.lock().unwrap() {
            return Err(SdioError::NotInitialized);
        }

        log::info!("Downloading firmware: {} bytes to 0x{:08x}", firmware_data.len(), base_address);
        
        let block_size = self.get_block_size();
        let blocks = (firmware_data.len() + block_size - 1) / block_size;
        
        for block_idx in 0..blocks {
            let start = block_idx * block_size;
            let end = std::cmp::min(start + block_size, firmware_data.len());
            let block_data = &firmware_data[start..end];
            let block_addr = base_address + (block_idx * block_size) as u32;
            
            self.write_multi_block(block_addr, block_data)?;
            
            log::debug!("Firmware block {}/{} written", block_idx + 1, blocks);
        }
        
        log::info!("Firmware download complete");
        Ok(())
    }

    pub fn fail_next_operation(&self) {
        *self.fail_next.lock().unwrap() = true;
    }

    pub fn clear_memory(&self) {
        self.memory.lock().unwrap().clear();
        log::info!("SDIO memory cleared");
    }

    pub fn get_memory_snapshot(&self) -> HashMap<u32, u8> {
        self.memory.lock().unwrap().clone()
    }

    pub fn get_transactions(&self) -> Vec<SdioTransaction> {
        self.transactions.lock().unwrap().clone()
    }

    pub fn clear_transactions(&self) {
        self.transactions.lock().unwrap().clear();
    }

    fn record_transaction(&self, address: u32, data: Vec<u8>, is_write: bool) {
        let mut counter = self.transaction_counter.lock().unwrap();
        *counter += 1;
        let timestamp = *counter;
        
        let transaction = SdioTransaction {
            address,
            data,
            is_write,
            timestamp,
        };
        
        self.transactions.lock().unwrap().push(transaction);
    }

    pub fn verify_firmware_at(&self, address: u32, expected: &[u8]) -> bool {
        let memory = self.memory.lock().unwrap();
        
        for (i, &expected_byte) in expected.iter().enumerate() {
            let addr = address.wrapping_add(i as u32);
            let actual_byte = memory.get(&addr).copied().unwrap_or(0);
            
            if actual_byte != expected_byte {
                log::error!(
                    "Firmware verification failed at offset {}: expected 0x{:02x}, got 0x{:02x}",
                    i, expected_byte, actual_byte
                );
                return false;
            }
        }
        
        true
    }
}

impl Default for MockSdioDevice {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        let device = MockSdioDevice::new();
        assert!(device.initialize().is_ok());
    }

    #[test]
    fn test_block_size() {
        let device = MockSdioDevice::new();
        device.initialize().unwrap();
        
        assert!(device.set_block_size(1024).is_ok());
        assert_eq!(device.get_block_size(), 1024);
    }

    #[test]
    fn test_byte_operations() {
        let device = MockSdioDevice::new();
        device.initialize().unwrap();
        
        assert!(device.write_byte(0x1000, 0x42).is_ok());
        assert_eq!(device.read_byte(0x1000).unwrap(), 0x42);
    }

    #[test]
    fn test_multi_block_operations() {
        let device = MockSdioDevice::new();
        device.initialize().unwrap();
        
        let data = vec![1, 2, 3, 4, 5];
        assert!(device.write_multi_block(0x2000, &data).is_ok());
        
        let read_data = device.read_multi_block(0x2000, data.len()).unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_firmware_download() {
        let device = MockSdioDevice::new();
        device.initialize().unwrap();
        device.set_block_size(512).unwrap();
        
        let firmware = vec![0xAA; 1024];
        assert!(device.download_firmware(0x00100000, &firmware).is_ok());
        
        assert!(device.verify_firmware_at(0x00100000, &firmware));
    }

    #[test]
    fn test_error_conditions() {
        let device = MockSdioDevice::new();
        
        assert_eq!(device.read_byte(0x1000), Err(SdioError::NotInitialized));
        assert_eq!(device.write_byte(0x1000, 0x42), Err(SdioError::NotInitialized));
        
        device.initialize().unwrap();
        device.fail_next_operation();
        assert_eq!(device.read_byte(0x1000), Err(SdioError::TransferError));
    }

    #[test]
    fn test_transaction_recording() {
        let device = MockSdioDevice::new();
        device.initialize().unwrap();
        
        device.write_byte(0x1000, 0x42).unwrap();
        device.read_byte(0x1000).unwrap();
        
        let transactions = device.get_transactions();
        assert_eq!(transactions.len(), 2);
        assert!(transactions[0].is_write);
        assert!(!transactions[1].is_write);
    }
}
