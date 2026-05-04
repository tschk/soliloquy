//! Browser configuration

use std::env;
use std::path::PathBuf;

/// Writable browser data directories for an immutable appliance layout.
///
/// The host stays read-only while browser state is confined to a narrow set
/// of explicit directories.
#[derive(Debug, Clone)]
pub struct BrowserDataDirs {
    pub profile_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub downloads_dir: PathBuf,
    pub state_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub terminal_state_dir: PathBuf,
}

impl BrowserDataDirs {
    fn env_dir(key: &str, fallback: &str) -> PathBuf {
        env::var_os(key)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(fallback))
    }

    /// Default writable locations for the browser appliance.
    pub fn appliance() -> Self {
        Self {
            profile_dir: Self::env_dir(
                "SOLILOQUY_PROFILE_DIR",
                "/var/lib/soliloquy/browser/profiles/default",
            ),
            cache_dir: Self::env_dir("SOLILOQUY_CACHE_DIR", "/var/lib/soliloquy/browser/cache"),
            downloads_dir: Self::env_dir(
                "SOLILOQUY_DOWNLOADS_DIR",
                "/var/lib/soliloquy/browser/downloads",
            ),
            state_dir: Self::env_dir("SOLILOQUY_STATE_DIR", "/var/lib/soliloquy/browser/state"),
            logs_dir: Self::env_dir("SOLILOQUY_LOG_DIR", "/var/lib/soliloquy/browser/logs"),
            terminal_state_dir: Self::env_dir(
                "SOLILOQUY_TERMINAL_STATE_DIR",
                "/var/lib/soliloquy/browser/terminal",
            ),
        }
    }
}

impl Default for BrowserDataDirs {
    fn default() -> Self {
        Self::appliance()
    }
}

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

    /// Explicit writable browser data directories for immutable deployments.
    pub data_dirs: BrowserDataDirs,

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
        let data_dirs = BrowserDataDirs::default();
        BrowserConfig {
            multi_process: true,
            gpu_compositing: true,
            hardware_acceleration: true,
            user_data_dir: data_dirs.profile_dir.clone(),
            data_dirs,
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
    /// Create settings optimized for the browser-only appliance.
    pub fn appliance() -> Self {
        let data_dirs = BrowserDataDirs::appliance();
        BrowserConfig {
            multi_process: true,
            gpu_compositing: true,
            hardware_acceleration: true,
            user_data_dir: data_dirs.profile_dir.clone(),
            data_dirs,
            max_renderers: 4,
            site_isolation: true,
            sandbox: true,
            window_width: 1280,
            window_height: 800,
            devtools_enabled: false,
            devtools_port: None,
            user_agent_override: None,
            disable_web_security: false,
            incognito: false,
            headless: false,
            v8_flags: vec!["--turbofan".to_string(), "--concurrent-marking".to_string()],
            cache_size_bytes: 128 * 1024 * 1024,
            enable_http3: true,
        }
    }

    /// Create a development configuration
    pub fn development() -> Self {
        let data_dirs = BrowserDataDirs::default();
        BrowserConfig {
            devtools_enabled: true,
            sandbox: false, // Easier debugging
            user_data_dir: data_dirs.profile_dir.clone(),
            data_dirs,
            ..Default::default()
        }
    }

    /// Create a headless configuration
    pub fn headless() -> Self {
        let data_dirs = BrowserDataDirs::default();
        BrowserConfig {
            headless: true,
            gpu_compositing: false,
            hardware_acceleration: false,
            devtools_port: Some(9222),
            user_data_dir: data_dirs.profile_dir.clone(),
            data_dirs,
            ..Default::default()
        }
    }

    /// Create an incognito configuration
    pub fn incognito() -> Self {
        let data_dirs = BrowserDataDirs::default();
        BrowserConfig {
            incognito: true,
            cache_size_bytes: 0,
            user_data_dir: data_dirs.profile_dir.clone(),
            data_dirs,
            ..Default::default()
        }
    }
}
