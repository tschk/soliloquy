use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareError {
    NotFound,
    InvalidSize,
    LoadFailed,
}

pub type FirmwareResult<T> = Result<T, FirmwareError>;

#[derive(Clone)]
pub struct MockFirmwareLoader {
    firmwares: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    load_count: Arc<Mutex<HashMap<String, usize>>>,
}

impl MockFirmwareLoader {
    pub fn new() -> Self {
        Self {
            firmwares: Arc::new(Mutex::new(HashMap::new())),
            load_count: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_firmware(&self, name: &str, data: Vec<u8>) {
        let mut firmwares = self.firmwares.lock().unwrap();
        firmwares.insert(name.to_string(), data);
        log::info!("Added firmware '{}' ({} bytes)", name, firmwares.get(name).unwrap().len());
    }

    pub fn load_firmware(&self, name: &str) -> FirmwareResult<Vec<u8>> {
        let firmwares = self.firmwares.lock().unwrap();
        
        match firmwares.get(name) {
            Some(data) => {
                let mut count = self.load_count.lock().unwrap();
                *count.entry(name.to_string()).or_insert(0) += 1;
                
                log::info!("Loaded firmware '{}' ({} bytes)", name, data.len());
                Ok(data.clone())
            }
            None => {
                log::error!("Firmware '{}' not found", name);
                Err(FirmwareError::NotFound)
            }
        }
    }

    pub fn get_load_count(&self, name: &str) -> usize {
        let count = self.load_count.lock().unwrap();
        count.get(name).copied().unwrap_or(0)
    }

    pub fn clear(&self) {
        self.firmwares.lock().unwrap().clear();
        self.load_count.lock().unwrap().clear();
        log::info!("Cleared all firmwares");
    }

    pub fn create_test_firmware(size: usize, pattern: u8) -> Vec<u8> {
        vec![pattern; size]
    }

    pub fn create_aic8800_firmware() -> Vec<u8> {
        let mut firmware = Vec::new();
        
        firmware.extend_from_slice(b"AIC8800");
        firmware.push(0x01);
        
        let header_size: u32 = 64;
        firmware.extend_from_slice(&header_size.to_le_bytes());
        
        let code_size: u32 = 4096;
        firmware.extend_from_slice(&code_size.to_le_bytes());
        
        let data_offset: u32 = 0x00100000;
        firmware.extend_from_slice(&data_offset.to_le_bytes());
        
        firmware.resize(header_size as usize, 0);
        
        firmware.extend_from_slice(&[0x90, 0x00, 0x00, 0xEA]);
        
        for i in 0..(code_size as usize - 4) {
            firmware.push((i % 256) as u8);
        }
        
        log::info!("Created AIC8800 test firmware ({} bytes)", firmware.len());
        firmware
    }
}

impl Default for MockFirmwareLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_load_firmware() {
        let loader = MockFirmwareLoader::new();
        let data = vec![1, 2, 3, 4, 5];
        
        loader.add_firmware("test.bin", data.clone());
        let loaded = loader.load_firmware("test.bin").unwrap();
        
        assert_eq!(loaded, data);
        assert_eq!(loader.get_load_count("test.bin"), 1);
    }

    #[test]
    fn test_firmware_not_found() {
        let loader = MockFirmwareLoader::new();
        
        assert_eq!(
            loader.load_firmware("nonexistent.bin"),
            Err(FirmwareError::NotFound)
        );
    }

    #[test]
    fn test_load_count() {
        let loader = MockFirmwareLoader::new();
        let data = vec![1, 2, 3];
        
        loader.add_firmware("test.bin", data);
        
        loader.load_firmware("test.bin").unwrap();
        loader.load_firmware("test.bin").unwrap();
        loader.load_firmware("test.bin").unwrap();
        
        assert_eq!(loader.get_load_count("test.bin"), 3);
    }

    #[test]
    fn test_create_test_firmware() {
        let firmware = MockFirmwareLoader::create_test_firmware(1024, 0xAA);
        
        assert_eq!(firmware.len(), 1024);
        assert!(firmware.iter().all(|&b| b == 0xAA));
    }

    #[test]
    fn test_create_aic8800_firmware() {
        let firmware = MockFirmwareLoader::create_aic8800_firmware();
        
        assert!(firmware.len() > 0);
        assert_eq!(&firmware[0..7], b"AIC8800");
    }

    #[test]
    fn test_clear() {
        let loader = MockFirmwareLoader::new();
        
        loader.add_firmware("test1.bin", vec![1, 2, 3]);
        loader.add_firmware("test2.bin", vec![4, 5, 6]);
        loader.load_firmware("test1.bin").unwrap();
        
        loader.clear();
        
        assert_eq!(loader.load_firmware("test1.bin"), Err(FirmwareError::NotFound));
        assert_eq!(loader.get_load_count("test1.bin"), 0);
    }
}
