//! V8 JavaScript engine optimizations

use log::{debug, info};

/// V8 optimization controller
pub struct V8Optimizations {
    /// Enable TurboFan
    pub turbofan: bool,
    /// Enable Sparkplug
    pub sparkplug: bool,
    /// Max heap size MB
    pub max_heap_mb: usize,
    /// Enable code caching
    pub code_caching: bool,
}

impl Default for V8Optimizations {
    fn default() -> Self {
        V8Optimizations {
            turbofan: true,
            sparkplug: true,
            max_heap_mb: 512,
            code_caching: true,
        }
    }
}

impl V8Optimizations {
    /// Apply optimizations to V8
    pub fn apply(&self) {
        info!(
            "Applying V8 optimizations: turbofan={}, sparkplug={}, heap={}MB",
            self.turbofan, self.sparkplug, self.max_heap_mb
        );
    }

    /// Generate V8 flags
    pub fn to_flags(&self) -> Vec<String> {
        let mut flags = Vec::new();

        if self.turbofan {
            flags.push("--turbofan".to_string());
        }
        if self.sparkplug {
            flags.push("--sparkplug".to_string());
        }
        flags.push(format!("--max-heap-size={}", self.max_heap_mb));

        if self.code_caching {
            flags.push("--allow-natives-syntax".to_string());
        }

        flags
    }
}
