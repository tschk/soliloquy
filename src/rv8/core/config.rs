//! Browser configuration

use std::path::PathBuf;

/// Browser configuration options
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    /// Enable multi-process mode (Chrome-like)
    pub multi_process: bool,

    /// Enable GPU compositing
    pub gpu_compositing: bool,

    /// Enable hardware acceleration
    pub hardware_acceleration: bool,

    /// User data directory for profiles, cache, etc.
    pub user_data_dir: PathBuf,

    /// Maximum number of renderer processes
    pub max_renderers: usize,

    /// Enable site isolation (one renderer per site)
    pub site_isolation: bool,

    /// Enable sandboxing for renderer processes
    pub sandbox: bool,

    /// Initial window width
    pub window_width: u32,

    /// Initial window height
    pub window_height: u32,

    /// Enable DevTools
    pub devtools_enabled: bool,

    /// DevTools port for remote debugging
    pub devtools_port: Option<u16>,

    /// User agent override
    pub user_agent_override: Option<String>,

    /// Disable web security (for testing only)
    pub disable_web_security: bool,

    /// Incognito mode (no persistent storage)
    pub incognito: bool,

    /// Headless mode (no UI)
    pub headless: bool,

    /// Enable V8 optimization flags
    pub v8_flags: Vec<String>,

    /// Maximum cache size in bytes
    pub cache_size_bytes: usize,

    /// Enable HTTP/3 (QUIC)
    pub enable_http3: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        BrowserConfig {
            multi_process: true,
            gpu_compositing: true,
            hardware_acceleration: true,
            user_data_dir: PathBuf::from(".rv8"),
            max_renderers: 10,
            site_isolation: true,
            sandbox: true,
            window_width: 1280,
            window_height: 800,
            devtools_enabled: true,
            devtools_port: Some(9222),
            user_agent_override: None,
            disable_web_security: false,
            incognito: false,
            headless: false,
            v8_flags: vec![
                "--turbofan".to_string(),
                "--max-heap-size=512".to_string(),
                "--concurrent-marking".to_string(),
            ],
            cache_size_bytes: 256 * 1024 * 1024, // 256 MB
            enable_http3: true,
        }
    }
}

impl BrowserConfig {
    /// Create a development configuration
    pub fn development() -> Self {
        BrowserConfig {
            devtools_enabled: true,
            sandbox: false, // Easier debugging
            ..Default::default()
        }
    }

    /// Create a headless configuration
    pub fn headless() -> Self {
        BrowserConfig {
            headless: true,
            gpu_compositing: false,
            hardware_acceleration: false,
            devtools_port: Some(9222),
            ..Default::default()
        }
    }

    /// Create an incognito configuration
    pub fn incognito() -> Self {
        BrowserConfig {
            incognito: true,
            cache_size_bytes: 0,
            ..Default::default()
        }
    }
}
