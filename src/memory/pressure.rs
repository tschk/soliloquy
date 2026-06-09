//! Memory pressure monitoring and response system
//!
//! Monitors system memory usage and triggers aggressive eviction when
//! memory pressure is detected.

use log::info;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressureLevel {
    /// Normal operation - no pressure
    Normal,
    /// Warning level - start conservative eviction
    Warning,
    /// Critical level - aggressive eviction required
    Critical,
}

/// Memory pressure monitor
pub struct MemoryPressureMonitor {
    /// Current memory usage in bytes
    current_usage: Arc<AtomicUsize>,
    /// Memory pressure threshold for warning level
    warning_threshold: usize,
    /// Memory pressure threshold for critical level
    critical_threshold: usize,
    /// Whether monitoring is active
    monitoring_active: Arc<AtomicBool>,
}

impl MemoryPressureMonitor {
    /// Create a new memory pressure monitor
    ///
    /// # Arguments
    /// * `warning_threshold` - Memory usage (bytes) to trigger warning level
    /// * `critical_threshold` - Memory usage (bytes) to trigger critical level
    pub fn new(warning_threshold: usize, critical_threshold: usize) -> Self {
        MemoryPressureMonitor {
            current_usage: Arc::new(AtomicUsize::new(0)),
            warning_threshold,
            critical_threshold,
            monitoring_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start monitoring memory pressure
    pub fn start_monitoring(&self) {
        self.monitoring_active.store(true, Ordering::SeqCst);
        info!("Memory pressure monitoring started");
        info!(
            "Warning threshold: {} MB",
            self.warning_threshold / 1024 / 1024
        );
        info!(
            "Critical threshold: {} MB",
            self.critical_threshold / 1024 / 1024
        );
    }

    /// Stop monitoring
    pub fn stop_monitoring(&self) {
        self.monitoring_active.store(false, Ordering::SeqCst);
        info!("Memory pressure monitoring stopped");
    }

    /// Update current memory usage
    pub fn update_usage(&self, usage: usize) {
        self.current_usage.store(usage, Ordering::SeqCst);
    }

    /// Get current memory pressure level
    pub fn get_pressure_level(&self) -> MemoryPressureLevel {
        let usage = self.current_usage.load(Ordering::SeqCst);

        if usage >= self.critical_threshold {
            MemoryPressureLevel::Critical
        } else if usage >= self.warning_threshold {
            MemoryPressureLevel::Warning
        } else {
            MemoryPressureLevel::Normal
        }
    }

    /// Check if under memory pressure
    pub fn is_under_pressure(&self) -> bool {
        self.get_pressure_level() != MemoryPressureLevel::Normal
    }

    /// Get memory usage as percentage of critical threshold
    pub fn get_usage_percentage(&self) -> f32 {
        let usage = self.current_usage.load(Ordering::SeqCst);
        if self.critical_threshold == 0 {
            return 0.0;
        }
        (usage as f32 / self.critical_threshold as f32) * 100.0
    }
}

impl Default for MemoryPressureMonitor {
    fn default() -> Self {
        // Default thresholds based on 3GB target
        Self::new(
            2 * 1024 * 1024 * 1024, // 2GB warning
            3 * 1024 * 1024 * 1024, // 3GB critical
        )
    }
}

pub struct SystemMemoryInfo {
    /// Total system memory in bytes
    pub total_memory: usize,
    /// Available memory in bytes
    pub available_memory: usize,
    /// Used memory in bytes
    pub used_memory: usize,
}

impl SystemMemoryInfo {
    /// Get current system memory information
    #[cfg(target_os = "linux")]
    pub fn get() -> Result<Self, String> {
        use std::fs;

        // Read /proc/meminfo
        let meminfo = fs::read_to_string("/proc/meminfo")
            .map_err(|e| format!("Failed to read /proc/meminfo: {}", e))?;

        let mut total_memory = 0;
        let mut available_memory = 0;

        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(value) = parse_meminfo_value(line) {
                    total_memory = value * 1024; // Convert KB to bytes
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(value) = parse_meminfo_value(line) {
                    available_memory = value * 1024; // Convert KB to bytes
                }
            }
        }

        if total_memory == 0 {
            return Err("Failed to parse MemTotal from /proc/meminfo".to_string());
        }

        let used_memory = total_memory.saturating_sub(available_memory);

        Ok(SystemMemoryInfo {
            total_memory,
            available_memory,
            used_memory,
        })
    }

    #[cfg(target_os = "macos")]
    pub fn get() -> Result<Self, String> {
        let total_memory = sysctl_usize("hw.memsize")?;
        let page_size = sysctl_usize("hw.pagesize")?;
        let vm_stat = std::process::Command::new("vm_stat")
            .output()
            .map_err(|e| format!("Failed to run vm_stat: {}", e))?;
        if !vm_stat.status.success() {
            return Err(format!("vm_stat exited with status {}", vm_stat.status));
        }
        let stdout = String::from_utf8(vm_stat.stdout)
            .map_err(|e| format!("Failed to decode vm_stat output: {}", e))?;

        let free_pages = parse_vm_stat_pages(&stdout, "Pages free:").unwrap_or(0);
        let inactive_pages = parse_vm_stat_pages(&stdout, "Pages inactive:").unwrap_or(0);
        let speculative_pages = parse_vm_stat_pages(&stdout, "Pages speculative:").unwrap_or(0);
        let available_memory = free_pages
            .saturating_add(inactive_pages)
            .saturating_add(speculative_pages)
            .saturating_mul(page_size)
            .min(total_memory);
        let used_memory = total_memory.saturating_sub(available_memory);

        Ok(SystemMemoryInfo {
            total_memory,
            available_memory,
            used_memory,
        })
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub fn get() -> Result<Self, String> {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();

        let total_memory = sys.total_memory() as usize;
        let available_memory = sys.available_memory() as usize;
        let used_memory = sys.used_memory() as usize;

        Ok(SystemMemoryInfo {
            total_memory,
            available_memory,
            used_memory,
        })
    }
}

/// Parse a value from /proc/meminfo line (e.g., "MemTotal:       16384000 kB")
#[cfg(target_os = "linux")]
fn parse_meminfo_value(line: &str) -> Option<usize> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse::<usize>().ok()
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn sysctl_usize(name: &str) -> Result<usize, String> {
    let output = std::process::Command::new("sysctl")
        .args(["-n", name])
        .output()
        .map_err(|e| format!("Failed to run sysctl {}: {}", name, e))?;
    if !output.status.success() {
        return Err(format!(
            "sysctl {} exited with status {}",
            name, output.status
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to decode sysctl {} output: {}", name, e))?;
    stdout
        .trim()
        .parse::<usize>()
        .map_err(|e| format!("Failed to parse sysctl {} output: {}", name, e))
}

#[cfg(target_os = "macos")]
fn parse_vm_stat_pages(output: &str, key: &str) -> Option<usize> {
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix(key))
        .and_then(|value| {
            value
                .trim()
                .trim_end_matches('.')
                .replace(',', "")
                .parse::<usize>()
                .ok()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pressure_levels() {
        let monitor = MemoryPressureMonitor::new(
            1024 * 1024 * 1024,     // 1GB warning
            2 * 1024 * 1024 * 1024, // 2GB critical
        );

        monitor.update_usage(512 * 1024 * 1024); // 512MB
        assert_eq!(monitor.get_pressure_level(), MemoryPressureLevel::Normal);

        monitor.update_usage(1536 * 1024 * 1024); // 1.5GB
        assert_eq!(monitor.get_pressure_level(), MemoryPressureLevel::Warning);

        monitor.update_usage(2560 * 1024 * 1024); // 2.5GB
        assert_eq!(monitor.get_pressure_level(), MemoryPressureLevel::Critical);
    }

    #[test]
    fn test_usage_percentage() {
        let monitor = MemoryPressureMonitor::new(1024 * 1024 * 1024, 2 * 1024 * 1024 * 1024);

        monitor.update_usage(1024 * 1024 * 1024); // 1GB
        let percentage = monitor.get_usage_percentage();
        assert!(percentage > 49.0 && percentage < 51.0); // ~50%
    }

    #[test]
    fn test_zero_critical_threshold_usage_percentage() {
        let monitor = MemoryPressureMonitor::new(0, 0);
        monitor.update_usage(1024);

        assert_eq!(monitor.get_usage_percentage(), 0.0);
    }

    #[test]
    fn test_monitoring_lifecycle() {
        let monitor = MemoryPressureMonitor::default();

        monitor.start_monitoring();
        assert!(monitor.monitoring_active.load(Ordering::SeqCst));

        monitor.stop_monitoring();
        assert!(!monitor.monitoring_active.load(Ordering::SeqCst));
    }

    #[test]
    fn test_system_memory_info() {
        let info = SystemMemoryInfo::get();
        assert!(info.is_ok());

        let info = info.unwrap();
        assert!(info.total_memory > 0);
        assert!(info.used_memory <= info.total_memory);
        assert!(info.available_memory <= info.total_memory);
    }
}
