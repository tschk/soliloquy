//! Persistent driver catalog built on sled.
//!
//! The runtime driver manager keeps policy and lifecycle state in memory, but
//! this catalog makes install/enable/disable/remove decisions durable so the
//! system can restore capability availability on the next boot.

use crate::driver_manager::{
    Capability, DriverError, DriverLease, DriverManager, DriverManifest, DriverRecord, DriverState,
    RequireSignedPackages, TrustPolicy,
};
use sled::Db;
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex};

const DRIVER_RECORDS_TREE: &str = "driver_records";
const CAPABILITY_SOURCES_TREE: &str = "capability_sources";
const CAPABILITY_SETTINGS_TREE: &str = "capability_settings";

#[derive(Debug)]
pub enum DriverCatalogError {
    Storage(sled::Error),
    Serialization(serde_json::Error),
    Driver(DriverError),
}

impl fmt::Display for DriverCatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(err) => write!(f, "storage error: {}", err),
            Self::Serialization(err) => write!(f, "serialization error: {}", err),
            Self::Driver(err) => write!(f, "driver error: {}", err),
        }
    }
}

impl std::error::Error for DriverCatalogError {}

impl From<sled::Error> for DriverCatalogError {
    fn from(value: sled::Error) -> Self {
        Self::Storage(value)
    }
}

impl From<serde_json::Error> for DriverCatalogError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value)
    }
}

impl From<DriverError> for DriverCatalogError {
    fn from(value: DriverError) -> Self {
        Self::Driver(value)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PersistedDriverRecord {
    record: DriverRecord,
}

impl From<DriverRecord> for PersistedDriverRecord {
    fn from(mut record: DriverRecord) -> Self {
        record.active_leases = 0;
        Self { record }
    }
}

impl From<PersistedDriverRecord> for DriverRecord {
    fn from(value: PersistedDriverRecord) -> Self {
        value.record
    }
}

/// Sled-backed store for driver records.
#[derive(Clone)]
pub struct DriverCatalog {
    db: Db,
    records: sled::Tree,
    capability_sources: sled::Tree,
    capability_settings: sled::Tree,
}

impl DriverCatalog {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DriverCatalogError> {
        let db = sled::open(path).map_err(DriverCatalogError::Storage)?;
        let records = db.open_tree(DRIVER_RECORDS_TREE)?;
        let capability_sources = db.open_tree(CAPABILITY_SOURCES_TREE)?;
        let capability_settings = db.open_tree(CAPABILITY_SETTINGS_TREE)?;
        Ok(Self {
            db,
            records,
            capability_sources,
            capability_settings,
        })
    }

    pub fn load_records(&self) -> Result<Vec<DriverRecord>, DriverCatalogError> {
        let mut loaded = Vec::new();

        for entry in self.records.iter() {
            let (_, value) = entry?;
            let persisted: PersistedDriverRecord = serde_json::from_slice(&value)?;
            loaded.push(persisted.into());
        }

        Ok(loaded)
    }

    pub fn save_record(&self, record: &DriverRecord) -> Result<(), DriverCatalogError> {
        let persisted: PersistedDriverRecord = record.clone().into();
        let bytes = serde_json::to_vec(&persisted)?;
        self.records
            .insert(persisted.record.manifest.id.as_bytes(), bytes)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn remove_record(&self, id: &str) -> Result<(), DriverCatalogError> {
        self.records.remove(id.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    pub fn clear(&self) -> Result<(), DriverCatalogError> {
        self.records.clear()?;
        self.capability_sources.clear()?;
        self.capability_settings.clear()?;
        self.db.flush()?;
        Ok(())
    }

    pub fn clear_records(&self) -> Result<(), DriverCatalogError> {
        self.records.clear()?;
        self.db.flush()?;
        Ok(())
    }

    pub fn register_capability_source(
        &self,
        capability: &Capability,
        source_uri: &str,
    ) -> Result<(), DriverCatalogError> {
        let key = serde_json::to_vec(capability)?;
        self.capability_sources.insert(key, source_uri.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    pub fn capability_source(
        &self,
        capability: &Capability,
    ) -> Result<Option<String>, DriverCatalogError> {
        let key = serde_json::to_vec(capability)?;
        let value = self.capability_sources.get(key)?;
        Ok(value.map(|bytes| String::from_utf8_lossy(&bytes).to_string()))
    }

    pub fn set_capability_enabled(
        &self,
        capability: &Capability,
        enabled: bool,
    ) -> Result<(), DriverCatalogError> {
        let key = serde_json::to_vec(capability)?;
        let value = if enabled {
            b"enabled".as_slice()
        } else {
            b"disabled".as_slice()
        };
        self.capability_settings.insert(key, value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn capability_enabled(
        &self,
        capability: &Capability,
    ) -> Result<Option<bool>, DriverCatalogError> {
        let key = serde_json::to_vec(capability)?;
        let value = self.capability_settings.get(key)?;
        Ok(value.map(|bytes| bytes.as_ref() == b"enabled"))
    }

    pub fn capability_settings(&self) -> Result<Vec<(Capability, bool)>, DriverCatalogError> {
        let mut entries = Vec::new();
        for item in self.capability_settings.iter() {
            let (key, value) = item?;
            let capability: Capability = serde_json::from_slice(&key)?;
            entries.push((capability, value.as_ref() == b"enabled"));
        }
        Ok(entries)
    }

    pub fn load_manifest_from_source(
        &self,
        source_uri: &str,
    ) -> Result<DriverManifest, DriverCatalogError> {
        let manifest = if let Some(path) = source_uri.strip_prefix("file://") {
            let data = std::fs::read_to_string(path).map_err(|e| {
                DriverCatalogError::Driver(DriverError::Serialization(e.to_string()))
            })?;
            serde_json::from_str(&data)?
        } else if source_uri.starts_with("http://") || source_uri.starts_with("https://") {
            let body = reqwest::blocking::get(source_uri)
                .map_err(|e| DriverCatalogError::Driver(DriverError::Serialization(e.to_string())))?
                .error_for_status()
                .map_err(|e| DriverCatalogError::Driver(DriverError::Serialization(e.to_string())))?
                .text()
                .map_err(|e| {
                    DriverCatalogError::Driver(DriverError::Serialization(e.to_string()))
                })?;
            serde_json::from_str(&body)?
        } else {
            return Err(DriverCatalogError::Driver(DriverError::Serialization(
                format!("unsupported package source URI: {}", source_uri),
            )));
        };

        Ok(manifest)
    }

    pub fn install_manifest_from_source(
        &self,
        source_uri: &str,
    ) -> Result<DriverRecord, DriverCatalogError> {
        let manifest = self.load_manifest_from_source(source_uri)?;
        let mut record = DriverRecord::new(manifest.clone());
        record.state = DriverState::Installed;
        self.save_record(&record)?;
        Ok(record)
    }

    pub fn sync_manager(&self, manager: &DriverManager) -> Result<(), DriverCatalogError> {
        self.clear_records()?;
        for record in manager.snapshot_records() {
            self.save_record(&record)?;
        }
        Ok(())
    }

    pub fn hydrate_manager(
        &self,
        trust_policy: Arc<dyn TrustPolicy>,
    ) -> Result<DriverManager, DriverCatalogError> {
        let records = self.load_records()?;
        Ok(DriverManager::from_records(records, trust_policy)?)
    }
}

/// Persistent wrapper around the in-memory driver manager.
pub struct PersistentDriverManager {
    catalog: DriverCatalog,
    manager: Arc<Mutex<DriverManager>>,
    held_leases: std::collections::HashMap<Capability, DriverLease>,
}

impl PersistentDriverManager {
    pub fn open(
        path: impl AsRef<Path>,
        trust_policy: Arc<dyn TrustPolicy>,
    ) -> Result<Self, DriverCatalogError> {
        let catalog = DriverCatalog::open(path)?;
        let manager = catalog.hydrate_manager(trust_policy)?;
        let manager = Arc::new(Mutex::new(manager));
        let mut wrapper = Self {
            catalog,
            manager,
            held_leases: std::collections::HashMap::new(),
        };
        wrapper.restore_enabled_capabilities()?;
        Ok(wrapper)
    }

    pub fn new_empty(
        path: impl AsRef<Path>,
        trust_policy: Arc<dyn TrustPolicy>,
    ) -> Result<Self, DriverCatalogError> {
        let catalog = DriverCatalog::open(path)?;
        let manager = Arc::new(Mutex::new(DriverManager::with_trust_policy(trust_policy)));
        let wrapper = Self {
            catalog,
            manager,
            held_leases: std::collections::HashMap::new(),
        };
        wrapper.sync()?;
        Ok(wrapper)
    }

    pub fn manager(&self) -> Arc<Mutex<DriverManager>> {
        Arc::clone(&self.manager)
    }

    pub fn manager_mut(&self) -> Arc<Mutex<DriverManager>> {
        Arc::clone(&self.manager)
    }

    pub fn sync(&self) -> Result<(), DriverCatalogError> {
        let manager = self.manager.lock().map_err(|_| {
            DriverCatalogError::Driver(DriverError::Serialization(
                "driver manager lock poisoned".into(),
            ))
        })?;
        self.catalog.sync_manager(&manager)
    }

    pub fn register_driver(&mut self, manifest: DriverManifest) -> Result<(), DriverCatalogError> {
        self.manager
            .lock()
            .map_err(|_| {
                DriverCatalogError::Driver(DriverError::Serialization(
                    "driver manager lock poisoned".into(),
                ))
            })?
            .register_driver(manifest)?;
        self.sync()
    }

    pub fn register_capability_source(
        &mut self,
        capability: Capability,
        source_uri: String,
    ) -> Result<(), DriverCatalogError> {
        self.catalog
            .register_capability_source(&capability, &source_uri)?;
        Ok(())
    }

    pub fn set_capability_enabled(
        &mut self,
        capability: Capability,
        enabled: bool,
    ) -> Result<(), DriverCatalogError> {
        self.catalog.set_capability_enabled(&capability, enabled)?;

        if enabled {
            if self.held_leases.contains_key(&capability) {
                return Ok(());
            }

            let lease = self.acquire_lease_for_capability(capability.clone())?;
            self.held_leases.insert(capability, lease);
            self.sync()?;
            return Ok(());
        }

        if let Some(lease) = self.held_leases.remove(&capability) {
            let _ = lease.release();
        }

        self.sync()
    }

    fn acquire_lease_for_capability(
        &self,
        capability: Capability,
    ) -> Result<DriverLease, DriverCatalogError> {
        let driver_id = {
            let mut manager = self.manager.lock().map_err(|_| {
                DriverCatalogError::Driver(DriverError::Serialization(
                    "driver manager lock poisoned".into(),
                ))
            })?;
            match manager.acquire_capability(capability.clone()) {
                Ok(driver_id) => driver_id,
                Err(DriverError::CapabilityUnavailable(_)) => {
                    let source = self.catalog.capability_source(&capability)?;
                    let Some(source_uri) = source else {
                        return Err(DriverCatalogError::Driver(
                            DriverError::CapabilityUnavailable(capability),
                        ));
                    };

                    let record = self.catalog.install_manifest_from_source(&source_uri)?;
                    manager.register_driver(record.manifest.clone())?;
                    manager.enable_driver(&record.manifest.id)?;
                    manager.acquire_capability(capability.clone())?
                }
                Err(err) => return Err(DriverCatalogError::Driver(err)),
            }
        };

        Ok(DriverLease::new(
            Arc::clone(&self.manager),
            driver_id,
            capability,
        ))
    }

    pub fn acquire_capability(
        &mut self,
        capability: Capability,
    ) -> Result<String, DriverCatalogError> {
        let acquire_result = {
            let mut manager = self.manager.lock().map_err(|_| {
                DriverCatalogError::Driver(DriverError::Serialization(
                    "driver manager lock poisoned".into(),
                ))
            })?;
            manager.acquire_capability(capability.clone())
        };

        match acquire_result {
            Ok(driver_id) => {
                self.sync()?;
                Ok(driver_id)
            }
            Err(DriverError::CapabilityUnavailable(_)) => {
                let source = self.catalog.capability_source(&capability)?;
                let Some(source_uri) = source else {
                    return Err(DriverCatalogError::Driver(
                        DriverError::CapabilityUnavailable(capability),
                    ));
                };

                let record = self.catalog.install_manifest_from_source(&source_uri)?;
                {
                    let mut manager = self.manager.lock().map_err(|_| {
                        DriverCatalogError::Driver(DriverError::Serialization(
                            "driver manager lock poisoned".into(),
                        ))
                    })?;
                    manager.register_driver(record.manifest.clone())?;
                    manager.enable_driver(&record.manifest.id)?;
                    let driver_id = manager.acquire_capability(capability)?;
                    drop(manager);
                    self.sync()?;
                    Ok(driver_id)
                }
            }
            Err(err) => Err(DriverCatalogError::Driver(err)),
        }
    }

    pub fn install_driver(&mut self, id: &str) -> Result<(), DriverCatalogError> {
        self.manager
            .lock()
            .map_err(|_| {
                DriverCatalogError::Driver(DriverError::Serialization(
                    "driver manager lock poisoned".into(),
                ))
            })?
            .install_driver(id)?;
        self.sync()
    }

    pub fn enable_driver(&mut self, id: &str) -> Result<(), DriverCatalogError> {
        self.manager
            .lock()
            .map_err(|_| {
                DriverCatalogError::Driver(DriverError::Serialization(
                    "driver manager lock poisoned".into(),
                ))
            })?
            .enable_driver(id)?;
        self.sync()
    }

    pub fn disable_driver(&mut self, id: &str) -> Result<(), DriverCatalogError> {
        self.manager
            .lock()
            .map_err(|_| {
                DriverCatalogError::Driver(DriverError::Serialization(
                    "driver manager lock poisoned".into(),
                ))
            })?
            .disable_driver(id)?;
        self.sync()
    }

    pub fn remove_driver(&mut self, id: &str) -> Result<(), DriverCatalogError> {
        self.manager
            .lock()
            .map_err(|_| {
                DriverCatalogError::Driver(DriverError::Serialization(
                    "driver manager lock poisoned".into(),
                ))
            })?
            .remove_driver(id)?;
        self.sync()
    }

    pub fn state(&self, id: &str) -> Option<DriverState> {
        self.manager
            .lock()
            .ok()
            .and_then(|manager| manager.state(id))
    }

    pub fn capability_enabled(
        &self,
        capability: &Capability,
    ) -> Result<Option<bool>, DriverCatalogError> {
        self.catalog.capability_enabled(capability)
    }

    pub fn restore_enabled_capabilities(&mut self) -> Result<(), DriverCatalogError> {
        let settings = self.catalog.capability_settings()?;
        for (capability, enabled) in settings {
            if enabled && !self.held_leases.contains_key(&capability) {
                let lease = self.acquire_lease_for_capability(capability.clone())?;
                self.held_leases.insert(capability, lease);
            }
        }
        self.sync()
    }
}

impl Default for PersistentDriverManager {
    fn default() -> Self {
        let catalog = DriverCatalog::open(std::env::temp_dir().join("soliloquy-driver-catalog"))
            .expect("temporary driver catalog");
        let manager = Arc::new(Mutex::new(DriverManager::with_trust_policy(Arc::new(
            RequireSignedPackages,
        ))));
        Self {
            catalog,
            manager,
            held_leases: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver_manager::{AllowUnsignedPackages, PackageSignature, RequireSignedPackages};
    use tempfile::tempdir;

    fn signed_manifest(id: &str, capability: Capability) -> DriverManifest {
        let mut manifest = DriverManifest::new(id, id, "1.0.0");
        manifest.capabilities.push(capability);
        let digest = manifest.signed_digest().expect("digest");
        manifest.signature = Some(PackageSignature {
            key_id: "test-key".to_string(),
            digest_sha256: digest,
        });
        manifest
    }

    #[test]
    fn persists_state_across_reopen() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("catalog");
        let trust_policy = Arc::new(RequireSignedPackages);

        {
            let mut manager = PersistentDriverManager::open(&path, trust_policy.clone())
                .unwrap_or_else(|_| {
                    PersistentDriverManager::new_empty(&path, trust_policy).unwrap()
                });
            manager
                .register_driver(signed_manifest("bluetooth", Capability::Bluetooth))
                .unwrap();
            manager.enable_driver("bluetooth").unwrap();
            assert_eq!(manager.state("bluetooth"), Some(DriverState::Enabled));
        }

        let reopened =
            PersistentDriverManager::open(&path, Arc::new(RequireSignedPackages)).unwrap();
        assert_eq!(reopened.state("bluetooth"), Some(DriverState::Enabled));
        let manager = reopened.manager();
        assert_eq!(manager.lock().unwrap().active_leases("bluetooth"), Some(0));
    }

    #[test]
    fn installs_from_file_source_on_demand() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("catalog");
        let trust_policy = Arc::new(AllowUnsignedPackages);
        let source_dir = tempdir.path().join("packages");
        std::fs::create_dir_all(&source_dir).unwrap();

        let manifest_path = source_dir.join("bluetooth.json");
        let manifest = signed_manifest("bluetooth", Capability::Bluetooth);
        std::fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();

        let mut manager = PersistentDriverManager::open(&path, trust_policy).unwrap_or_else(|_| {
            PersistentDriverManager::new_empty(&path, Arc::new(AllowUnsignedPackages)).unwrap()
        });
        manager
            .register_capability_source(
                Capability::Bluetooth,
                format!("file://{}", manifest_path.display()),
            )
            .unwrap();

        let driver_id = manager.acquire_capability(Capability::Bluetooth).unwrap();
        assert_eq!(driver_id, "bluetooth");
        assert_eq!(manager.state("bluetooth"), Some(DriverState::Enabled));
    }

    #[test]
    fn settings_toggle_persists_enabled_state() {
        let tempdir = tempdir().expect("tempdir");
        let path = tempdir.path().join("catalog");
        let trust_policy = Arc::new(AllowUnsignedPackages);
        let source_dir = tempdir.path().join("packages");
        std::fs::create_dir_all(&source_dir).unwrap();

        let manifest_path = source_dir.join("bluetooth.json");
        let manifest = signed_manifest("bluetooth", Capability::Bluetooth);
        std::fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();

        {
            let mut manager = PersistentDriverManager::open(&path, trust_policy.clone())
                .unwrap_or_else(|_| {
                    PersistentDriverManager::new_empty(&path, trust_policy).unwrap()
                });
            manager
                .register_capability_source(
                    Capability::Bluetooth,
                    format!("file://{}", manifest_path.display()),
                )
                .unwrap();
            manager
                .set_capability_enabled(Capability::Bluetooth, true)
                .unwrap();
            assert_eq!(
                manager.capability_enabled(&Capability::Bluetooth).unwrap(),
                Some(true)
            );
            assert_eq!(manager.state("bluetooth"), Some(DriverState::Enabled));
        }

        let reopened =
            PersistentDriverManager::open(&path, Arc::new(AllowUnsignedPackages)).unwrap();
        assert_eq!(
            reopened.capability_enabled(&Capability::Bluetooth).unwrap(),
            Some(true)
        );
        assert_eq!(reopened.state("bluetooth"), Some(DriverState::Enabled));
        let manager = reopened.manager();
        assert_eq!(manager.lock().unwrap().active_leases("bluetooth"), Some(1));
    }
}
