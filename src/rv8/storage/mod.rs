//! Storage subsystems module

use log::info;
use std::path::Path;

/// Storage manager for profiles, cache, cookies
pub struct StorageManager {
    // sled database will go here
}

impl StorageManager {
    pub async fn new(data_dir: &Path) -> Result<Self, String> {
        info!("Initializing storage at {:?}", data_dir);
        Ok(StorageManager {})
    }

    pub async fn flush(&self) {
        info!("Flushing storage");
    }
}

/// Cookie jar for managing cookies
pub struct CookieJar {
    // Cookies will be stored here
}

impl CookieJar {
    pub fn new() -> Self {
        CookieJar {}
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}
