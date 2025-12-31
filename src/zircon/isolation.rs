//! Capability-based security isolation for tabs
//!
//! Each tab runs in an isolated environment with explicit capability grants.
//! This prevents tabs from accessing resources they shouldn't have.

use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::Mutex;

/// Capability types that can be granted to tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Read files from disk
    FileRead,
    /// Write files to disk
    FileWrite,
    /// Access network
    Network,
    /// Access camera
    Camera,
    /// Access microphone
    Microphone,
    /// Access geolocation
    Geolocation,
    /// Access clipboard
    Clipboard,
    /// Access notifications
    Notifications,
    /// Access local storage
    LocalStorage,
    /// Access IndexedDB
    IndexedDB,
    /// Access WebGL
    WebGL,
    /// Access WebGPU (WGPU)
    WebGPU,
}

impl Capability {
    /// Get human-readable name for the capability
    pub fn name(&self) -> &str {
        match self {
            Capability::FileRead => "File Read",
            Capability::FileWrite => "File Write",
            Capability::Network => "Network Access",
            Capability::Camera => "Camera",
            Capability::Microphone => "Microphone",
            Capability::Geolocation => "Geolocation",
            Capability::Clipboard => "Clipboard",
            Capability::Notifications => "Notifications",
            Capability::LocalStorage => "Local Storage",
            Capability::IndexedDB => "IndexedDB",
            Capability::WebGL => "WebGL",
            Capability::WebGPU => "WebGPU",
        }
    }

    /// Get default capabilities granted to all tabs
    pub fn defaults() -> Vec<Capability> {
        vec![
            Capability::Network,
            Capability::LocalStorage,
            Capability::IndexedDB,
            Capability::WebGL,
            Capability::WebGPU,
        ]
    }
}

/// Permission status for a capability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionStatus {
    /// Capability granted
    Granted,
    /// Capability denied
    Denied,
    /// User should be prompted
    Prompt,
}

/// Tab isolation context
pub struct TabIsolation {
    /// Tab identifier
    tab_id: u64,
    /// Granted capabilities
    capabilities: HashMap<Capability, PermissionStatus>,
    /// Origin (scheme + host + port) for permission scoping
    origin: String,
}

impl TabIsolation {
    /// Create a new isolation context for a tab
    pub fn new(tab_id: u64, origin: String) -> Self {
        let mut capabilities = HashMap::new();
        
        // Grant default capabilities
        for cap in Capability::defaults() {
            capabilities.insert(cap, PermissionStatus::Granted);
        }

        info!("Created isolation context for tab {} (origin: {})", tab_id, origin);

        TabIsolation {
            tab_id,
            capabilities,
            origin,
        }
    }

    /// Check if a capability is granted
    pub fn has_capability(&self, capability: Capability) -> bool {
        self.capabilities.get(&capability)
            .map(|status| *status == PermissionStatus::Granted)
            .unwrap_or(false)
    }

    /// Grant a capability
    pub fn grant_capability(&mut self, capability: Capability) {
        debug!("Tab {}: Granting capability {:?}", self.tab_id, capability);
        self.capabilities.insert(capability, PermissionStatus::Granted);
    }

    /// Revoke a capability
    pub fn revoke_capability(&mut self, capability: Capability) {
        debug!("Tab {}: Revoking capability {:?}", self.tab_id, capability);
        self.capabilities.insert(capability, PermissionStatus::Denied);
    }

    /// Get permission status for a capability
    pub fn get_permission(&self, capability: Capability) -> PermissionStatus {
        self.capabilities.get(&capability)
            .copied()
            .unwrap_or(PermissionStatus::Prompt)
    }

    /// Get the origin for this tab
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// Get tab ID
    pub fn tab_id(&self) -> u64 {
        self.tab_id
    }
}

/// Isolation manager for all tabs
pub struct IsolationManager {
    /// Isolation contexts indexed by tab ID
    contexts: Mutex<HashMap<u64, TabIsolation>>,
}

impl IsolationManager {
    /// Create a new isolation manager
    pub fn new() -> Self {
        IsolationManager {
            contexts: Mutex::new(HashMap::new()),
        }
    }

    /// Create isolation context for a new tab
    pub fn create_context(&self, tab_id: u64, origin: String) -> Result<(), String> {
        let mut contexts = self.contexts.lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        if contexts.contains_key(&tab_id) {
            return Err(format!("Tab {} already has isolation context", tab_id));
        }

        let context = TabIsolation::new(tab_id, origin);
        contexts.insert(tab_id, context);

        Ok(())
    }

    /// Remove isolation context
    pub fn remove_context(&self, tab_id: u64) -> Result<(), String> {
        let mut contexts = self.contexts.lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        contexts.remove(&tab_id)
            .ok_or_else(|| format!("Tab {} has no isolation context", tab_id))?;

        info!("Removed isolation context for tab {}", tab_id);
        Ok(())
    }

    /// Check if tab has a capability
    pub fn check_capability(&self, tab_id: u64, capability: Capability) -> Result<bool, String> {
        let contexts = self.contexts.lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let context = contexts.get(&tab_id)
            .ok_or_else(|| format!("Tab {} has no isolation context", tab_id))?;

        Ok(context.has_capability(capability))
    }

    /// Grant capability to a tab
    pub fn grant_capability(&self, tab_id: u64, capability: Capability) -> Result<(), String> {
        let mut contexts = self.contexts.lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let context = contexts.get_mut(&tab_id)
            .ok_or_else(|| format!("Tab {} has no isolation context", tab_id))?;

        context.grant_capability(capability);
        Ok(())
    }

    /// Revoke capability from a tab
    pub fn revoke_capability(&self, tab_id: u64, capability: Capability) -> Result<(), String> {
        let mut contexts = self.contexts.lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let context = contexts.get_mut(&tab_id)
            .ok_or_else(|| format!("Tab {} has no isolation context", tab_id))?;

        context.revoke_capability(capability);
        Ok(())
    }

    /// Get permission status
    pub fn get_permission(&self, tab_id: u64, capability: Capability) -> Result<PermissionStatus, String> {
        let contexts = self.contexts.lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let context = contexts.get(&tab_id)
            .ok_or_else(|| format!("Tab {} has no isolation context", tab_id))?;

        Ok(context.get_permission(capability))
    }
}

impl Default for IsolationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_defaults() {
        let defaults = Capability::defaults();
        assert!(defaults.contains(&Capability::Network));
        assert!(defaults.contains(&Capability::WebGPU));
    }

    #[test]
    fn test_tab_isolation_creation() {
        let isolation = TabIsolation::new(1, "https://example.com".to_string());
        assert_eq!(isolation.tab_id(), 1);
        assert_eq!(isolation.origin(), "https://example.com");
    }

    #[test]
    fn test_default_capabilities() {
        let isolation = TabIsolation::new(1, "https://example.com".to_string());
        
        // Default capabilities should be granted
        assert!(isolation.has_capability(Capability::Network));
        assert!(isolation.has_capability(Capability::WebGPU));
        
        // Non-default capabilities should not be granted
        assert!(!isolation.has_capability(Capability::Camera));
    }

    #[test]
    fn test_grant_revoke_capability() {
        let mut isolation = TabIsolation::new(1, "https://example.com".to_string());
        
        // Grant camera
        isolation.grant_capability(Capability::Camera);
        assert!(isolation.has_capability(Capability::Camera));
        
        // Revoke camera
        isolation.revoke_capability(Capability::Camera);
        assert!(!isolation.has_capability(Capability::Camera));
    }

    #[test]
    fn test_permission_status() {
        let mut isolation = TabIsolation::new(1, "https://example.com".to_string());
        
        assert_eq!(isolation.get_permission(Capability::Network), PermissionStatus::Granted);
        assert_eq!(isolation.get_permission(Capability::Camera), PermissionStatus::Prompt);
        
        isolation.revoke_capability(Capability::Network);
        assert_eq!(isolation.get_permission(Capability::Network), PermissionStatus::Denied);
    }

    #[test]
    fn test_isolation_manager() {
        let manager = IsolationManager::new();
        
        let result = manager.create_context(1, "https://example.com".to_string());
        assert!(result.is_ok());
        
        let has_cap = manager.check_capability(1, Capability::Network).unwrap();
        assert!(has_cap);
    }

    #[test]
    fn test_manager_grant_revoke() {
        let manager = IsolationManager::new();
        manager.create_context(1, "https://example.com".to_string()).unwrap();
        
        // Grant camera
        manager.grant_capability(1, Capability::Camera).unwrap();
        assert!(manager.check_capability(1, Capability::Camera).unwrap());
        
        // Revoke camera
        manager.revoke_capability(1, Capability::Camera).unwrap();
        assert!(!manager.check_capability(1, Capability::Camera).unwrap());
    }

    #[test]
    fn test_manager_remove_context() {
        let manager = IsolationManager::new();
        manager.create_context(1, "https://example.com".to_string()).unwrap();
        
        let result = manager.remove_context(1);
        assert!(result.is_ok());
        
        let result = manager.check_capability(1, Capability::Network);
        assert!(result.is_err());
    }
}
