//! Consumer-facing settings toggles for capability-backed services.
//!
//! This sits on top of the persistent driver manager so UI code can expose a
//! simple on/off switch for Bluetooth, Wi-Fi, camera, and similar features
//! without surfacing package installation details to the user.

use crate::driver_catalog::{DriverCatalogError, PersistentDriverManager};
use crate::driver_manager::{Capability, RequireSignedPackages};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingToggle {
    pub key: String,
    pub label: String,
    pub description: String,
    pub capability: Capability,
    pub enabled: bool,
}

impl SettingToggle {
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
        capability: Capability,
        enabled: bool,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            description: description.into(),
            capability,
            enabled,
        }
    }
}

pub struct SettingsManager {
    driver_manager: PersistentDriverManager,
    toggles: HashMap<String, SettingToggle>,
}

impl SettingsManager {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DriverCatalogError> {
        let driver_manager = PersistentDriverManager::open(path, Arc::new(RequireSignedPackages))?;
        Ok(Self {
            driver_manager,
            toggles: HashMap::new(),
        })
    }

    pub fn register_toggle(&mut self, toggle: SettingToggle) {
        self.toggles.insert(toggle.key.clone(), toggle);
    }

    pub fn toggle(&mut self, key: &str, enabled: bool) -> Result<(), DriverCatalogError> {
        let toggle = self.toggles.get_mut(key).ok_or_else(|| {
            DriverCatalogError::Driver(crate::driver_manager::DriverError::UnknownDriver(
                key.to_string(),
            ))
        })?;

        self.driver_manager
            .set_capability_enabled(toggle.capability.clone(), enabled)?;
        toggle.enabled = enabled;
        Ok(())
    }

    pub fn is_enabled(&self, key: &str) -> Option<bool> {
        self.toggles.get(key).map(|toggle| toggle.enabled)
    }

    pub fn restore_from_persistence(&mut self) -> Result<(), DriverCatalogError> {
        for toggle in self.toggles.values_mut() {
            if let Some(enabled) = self.driver_manager.capability_enabled(&toggle.capability)? {
                toggle.enabled = enabled;
            }
        }
        Ok(())
    }

    pub fn driver_manager(&self) -> &PersistentDriverManager {
        &self.driver_manager
    }

    pub fn driver_manager_mut(&mut self) -> &mut PersistentDriverManager {
        &mut self.driver_manager
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        let driver_manager = PersistentDriverManager::default();
        Self {
            driver_manager,
            toggles: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver_manager::{DriverManifest, PackageSignature};
    use tempfile::tempdir;

    #[test]
    fn toggling_updates_persistent_driver_manager() {
        let tempdir = tempdir().unwrap();
        let catalog_path = tempdir.path().join("settings");
        let mut settings = SettingsManager::open(&catalog_path).unwrap();

        let mut manifest = DriverManifest::new("bluetooth", "Bluetooth", "1.0.0");
        manifest.capabilities.push(Capability::Bluetooth);
        let digest = manifest.signed_digest().unwrap();
        manifest.signature = Some(PackageSignature {
            key_id: "test-key".to_string(),
            digest_sha256: digest,
        });
        settings
            .driver_manager_mut()
            .register_driver(manifest)
            .unwrap();

        settings.register_toggle(SettingToggle::new(
            "bluetooth",
            "Bluetooth",
            "Enable Bluetooth hardware and services",
            Capability::Bluetooth,
            false,
        ));

        settings.toggle("bluetooth", true).unwrap();
        assert_eq!(settings.is_enabled("bluetooth"), Some(true));
        assert_eq!(
            settings.driver_manager().state("bluetooth"),
            Some(crate::driver_manager::DriverState::Enabled)
        );

        settings.toggle("bluetooth", false).unwrap();
        assert_eq!(settings.is_enabled("bluetooth"), Some(false));
        assert_eq!(
            settings.driver_manager().state("bluetooth"),
            Some(crate::driver_manager::DriverState::Disabled)
        );
    }

    #[test]
    fn test_is_enabled_edge_cases() {
        let tempdir = tempdir().unwrap();
        let catalog_path = tempdir.path().join("settings");
        let settings = SettingsManager::open(&catalog_path).unwrap();

        assert_eq!(settings.is_enabled("nonexistent_key"), None);
        assert_eq!(settings.is_enabled(""), None);
    }
}
