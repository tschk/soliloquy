//! Runtime driver lifecycle support.
//!
//! This module models drivers as installable, enable-on-demand packages with
//! explicit capabilities, dependency resolution, and reversible removal.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

/// User-facing capabilities that can trigger driver activation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    Bluetooth,
    Wifi,
    Audio,
    Camera,
    Microphone,
    Location,
    Storage,
    Sensors,
    Gpio,
    Display,
}

/// Signed manifest metadata for a driver package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSignature {
    pub key_id: String,
    pub digest_sha256: String,
}

/// Driver package description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<Capability>,
    pub dependencies: Vec<String>,
    pub firmware: Vec<String>,
    pub package_uri: Option<String>,
    pub signature: Option<PackageSignature>,
    pub removable: bool,
}

impl DriverManifest {
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            capabilities: Vec::new(),
            dependencies: Vec::new(),
            firmware: Vec::new(),
            package_uri: None,
            signature: None,
            removable: true,
        }
    }

    pub fn signed_digest(&self) -> Result<String, DriverError> {
        let mut copy = self.clone();
        copy.signature = None;
        let bytes =
            serde_json::to_vec(&copy).map_err(|e| DriverError::Serialization(e.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Ok(format!("{:x}", hasher.finalize()))
    }
}

/// Lifecycle state for a driver package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriverState {
    Registered,
    Installed,
    Enabled,
    Disabled,
    Removed,
}

/// Mutable record tracked by the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverRecord {
    pub manifest: DriverManifest,
    pub state: DriverState,
    pub active_leases: usize,
}

impl DriverRecord {
    pub(crate) fn new(manifest: DriverManifest) -> Self {
        Self {
            manifest,
            state: DriverState::Registered,
            active_leases: 0,
        }
    }
}

/// Errors produced by the driver manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverError {
    DuplicateDriver(String),
    UnknownDriver(String),
    MissingDependency {
        driver_id: String,
        dependency_id: String,
    },
    DependencyInUse {
        driver_id: String,
        dependent_id: String,
    },
    CapabilityUnavailable(Capability),
    SignatureMissing(String),
    SignatureMismatch(String),
    NotInstalled(String),
    NotEnabled(String),
    ActiveConsumers {
        driver_id: String,
        active_leases: usize,
    },
    Serialization(String),
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateDriver(id) => write!(f, "driver {} already registered", id),
            Self::UnknownDriver(id) => write!(f, "unknown driver {}", id),
            Self::MissingDependency {
                driver_id,
                dependency_id,
            } => write!(
                f,
                "driver {} requires dependency {}",
                driver_id, dependency_id
            ),
            Self::DependencyInUse {
                driver_id,
                dependent_id,
            } => write!(
                f,
                "driver {} is still required by {}",
                driver_id, dependent_id
            ),
            Self::CapabilityUnavailable(capability) => {
                write!(f, "no driver registered for capability {:?}", capability)
            }
            Self::SignatureMissing(id) => write!(f, "driver {} has no signature", id),
            Self::SignatureMismatch(id) => write!(f, "driver {} signature mismatch", id),
            Self::NotInstalled(id) => write!(f, "driver {} is not installed", id),
            Self::NotEnabled(id) => write!(f, "driver {} is not enabled", id),
            Self::ActiveConsumers {
                driver_id,
                active_leases,
            } => write!(
                f,
                "driver {} still has {} active consumers",
                driver_id, active_leases
            ),
            Self::Serialization(msg) => write!(f, "serialization error: {}", msg),
        }
    }
}

impl std::error::Error for DriverError {}

/// Pluggable trust policy for package signatures.
pub trait TrustPolicy: Send + Sync {
    fn verify(&self, manifest: &DriverManifest) -> Result<(), DriverError>;
}

/// Strict policy that requires a matching signature digest.
#[derive(Debug, Default)]
pub struct RequireSignedPackages;

impl TrustPolicy for RequireSignedPackages {
    fn verify(&self, manifest: &DriverManifest) -> Result<(), DriverError> {
        let signature = manifest
            .signature
            .as_ref()
            .ok_or_else(|| DriverError::SignatureMissing(manifest.id.clone()))?;

        let expected = manifest.signed_digest()?;
        if signature.digest_sha256 != expected {
            return Err(DriverError::SignatureMismatch(manifest.id.clone()));
        }

        Ok(())
    }
}

/// Developer-friendly policy for local testing.
#[derive(Debug, Default)]
pub struct AllowUnsignedPackages;

impl TrustPolicy for AllowUnsignedPackages {
    fn verify(&self, _manifest: &DriverManifest) -> Result<(), DriverError> {
        Ok(())
    }
}

/// Registry of known drivers and their package state.
#[derive(Debug, Default)]
pub struct DriverRegistry {
    records: HashMap<String, DriverRecord>,
}

impl DriverRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_manifest(&mut self, manifest: DriverManifest) -> Result<(), DriverError> {
        if self.records.contains_key(&manifest.id) {
            return Err(DriverError::DuplicateDriver(manifest.id));
        }

        self.records
            .insert(manifest.id.clone(), DriverRecord::new(manifest));
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&DriverRecord> {
        self.records.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut DriverRecord> {
        self.records.get_mut(id)
    }

    pub fn insert_record(&mut self, record: DriverRecord) -> Result<(), DriverError> {
        if self.records.contains_key(&record.manifest.id) {
            return Err(DriverError::DuplicateDriver(record.manifest.id));
        }

        self.records.insert(record.manifest.id.clone(), record);
        Ok(())
    }

    pub fn ids_for_capability(&self, capability: &Capability) -> Vec<String> {
        self.records
            .values()
            .filter(|record| record.manifest.capabilities.contains(capability))
            .map(|record| record.manifest.id.clone())
            .collect()
    }

    pub fn records(&self) -> Vec<DriverRecord> {
        self.records.values().cloned().collect()
    }

    pub fn dependents_of(&self, id: &str) -> Vec<String> {
        self.records
            .values()
            .filter(|record| {
                record
                    .manifest
                    .dependencies
                    .iter()
                    .any(|dependency| dependency == id)
                    && record.state != DriverState::Removed
            })
            .map(|record| record.manifest.id.clone())
            .collect()
    }
}

/// Manager that owns the driver lifecycle logic.
pub struct DriverManager {
    registry: DriverRegistry,
    trust_policy: Arc<dyn TrustPolicy>,
    auto_disable_on_release: bool,
}

impl DriverManager {
    pub fn new() -> Self {
        Self {
            registry: DriverRegistry::new(),
            trust_policy: Arc::new(RequireSignedPackages),
            auto_disable_on_release: true,
        }
    }

    pub fn with_trust_policy(trust_policy: Arc<dyn TrustPolicy>) -> Self {
        Self {
            registry: DriverRegistry::new(),
            trust_policy,
            auto_disable_on_release: true,
        }
    }

    pub fn from_records(
        records: Vec<DriverRecord>,
        trust_policy: Arc<dyn TrustPolicy>,
    ) -> Result<Self, DriverError> {
        let mut registry = DriverRegistry::new();

        for mut record in records {
            trust_policy.verify(&record.manifest)?;
            record.active_leases = 0;
            registry.insert_record(record)?;
        }

        Ok(Self {
            registry,
            trust_policy,
            auto_disable_on_release: true,
        })
    }

    pub fn register_driver(&mut self, manifest: DriverManifest) -> Result<(), DriverError> {
        self.registry.register_manifest(manifest)
    }

    pub fn install_driver(&mut self, id: &str) -> Result<(), DriverError> {
        let dependencies = {
            let record = self
                .registry
                .get(id)
                .ok_or_else(|| DriverError::UnknownDriver(id.to_string()))?;
            self.trust_policy.verify(&record.manifest)?;
            record.manifest.dependencies.clone()
        };

        for dependency in dependencies {
            if self.registry.get(&dependency).is_none() {
                return Err(DriverError::MissingDependency {
                    driver_id: id.to_string(),
                    dependency_id: dependency,
                });
            }
            self.install_driver(&dependency)?;
        }

        let record = self
            .registry
            .get_mut(id)
            .ok_or_else(|| DriverError::UnknownDriver(id.to_string()))?;
        record.state = DriverState::Installed;
        Ok(())
    }

    pub fn enable_driver(&mut self, id: &str) -> Result<(), DriverError> {
        self.install_driver(id)?;

        let record = self
            .registry
            .get_mut(id)
            .ok_or_else(|| DriverError::UnknownDriver(id.to_string()))?;
        record.state = DriverState::Enabled;
        Ok(())
    }

    pub fn disable_driver(&mut self, id: &str) -> Result<(), DriverError> {
        let record = self
            .registry
            .get_mut(id)
            .ok_or_else(|| DriverError::UnknownDriver(id.to_string()))?;

        if record.active_leases > 0 {
            return Err(DriverError::ActiveConsumers {
                driver_id: id.to_string(),
                active_leases: record.active_leases,
            });
        }

        if record.state == DriverState::Removed {
            return Err(DriverError::NotInstalled(id.to_string()));
        }

        record.state = DriverState::Disabled;
        Ok(())
    }

    pub fn remove_driver(&mut self, id: &str) -> Result<(), DriverError> {
        let dependents = self.registry.dependents_of(id);
        if let Some(dependent_id) = dependents.first() {
            return Err(DriverError::DependencyInUse {
                driver_id: id.to_string(),
                dependent_id: dependent_id.clone(),
            });
        }

        let record = self
            .registry
            .get_mut(id)
            .ok_or_else(|| DriverError::UnknownDriver(id.to_string()))?;

        if record.active_leases > 0 {
            return Err(DriverError::ActiveConsumers {
                driver_id: id.to_string(),
                active_leases: record.active_leases,
            });
        }

        record.state = DriverState::Removed;
        Ok(())
    }

    pub fn resolve_capability(&self, capability: &Capability) -> Result<String, DriverError> {
        self.registry
            .ids_for_capability(capability)
            .into_iter()
            .next()
            .ok_or_else(|| DriverError::CapabilityUnavailable(capability.clone()))
    }

    pub fn acquire_capability(&mut self, capability: Capability) -> Result<String, DriverError> {
        let driver_id = self.resolve_capability(&capability)?;
        self.enable_driver(&driver_id)?;

        let record = self
            .registry
            .get_mut(&driver_id)
            .ok_or_else(|| DriverError::UnknownDriver(driver_id.clone()))?;
        record.active_leases += 1;
        Ok(driver_id)
    }

    pub fn release_capability(&mut self, driver_id: &str) -> Result<(), DriverError> {
        let record = self
            .registry
            .get_mut(driver_id)
            .ok_or_else(|| DriverError::UnknownDriver(driver_id.to_string()))?;

        if record.active_leases == 0 {
            return Err(DriverError::NotEnabled(driver_id.to_string()));
        }

        record.active_leases -= 1;
        if record.active_leases == 0 && self.auto_disable_on_release {
            record.state = DriverState::Disabled;
        }

        Ok(())
    }

    pub fn state(&self, id: &str) -> Option<DriverState> {
        self.registry.get(id).map(|record| record.state)
    }

    pub fn active_leases(&self, id: &str) -> Option<usize> {
        self.registry.get(id).map(|record| record.active_leases)
    }

    pub fn snapshot_records(&self) -> Vec<DriverRecord> {
        self.registry.records()
    }
}

impl Default for DriverManager {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII lease for a capability-backed driver activation.
pub struct DriverLease {
    manager: Arc<Mutex<DriverManager>>,
    driver_id: String,
    capability: Capability,
    released: bool,
}

impl DriverLease {
    pub(crate) fn new(
        manager: Arc<Mutex<DriverManager>>,
        driver_id: String,
        capability: Capability,
    ) -> Self {
        Self {
            manager,
            driver_id,
            capability,
            released: false,
        }
    }

    pub fn driver_id(&self) -> &str {
        &self.driver_id
    }

    pub fn capability(&self) -> &Capability {
        &self.capability
    }

    pub fn release(mut self) -> Result<(), DriverError> {
        self.released = true;
        let mut manager = self
            .manager
            .lock()
            .map_err(|_| DriverError::Serialization("driver manager lock poisoned".into()))?;
        manager.release_capability(&self.driver_id)
    }
}

impl Drop for DriverLease {
    fn drop(&mut self) {
        if self.released {
            return;
        }

        if let Ok(mut manager) = self.manager.lock() {
            let _ = manager.release_capability(&self.driver_id);
        }
    }
}

/// Broker that translates capability requests into driver leases.
#[derive(Clone, Default)]
pub struct CapabilityBroker {
    manager: Arc<Mutex<DriverManager>>,
}

impl CapabilityBroker {
    pub fn new(manager: DriverManager) -> Self {
        Self {
            manager: Arc::new(Mutex::new(manager)),
        }
    }

    pub fn manager(&self) -> Arc<Mutex<DriverManager>> {
        Arc::clone(&self.manager)
    }

    pub fn acquire(&self, capability: Capability) -> Result<DriverLease, DriverError> {
        let mut manager = self
            .manager
            .lock()
            .map_err(|_| DriverError::Serialization("driver manager lock poisoned".into()))?;
        let driver_id = manager.acquire_capability(capability.clone())?;
        Ok(DriverLease::new(
            Arc::clone(&self.manager),
            driver_id,
            capability,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_manifest_creation() {
        let manifest = DriverManifest::new("", "", "");
        assert_eq!(manifest.id, "");
        assert_eq!(manifest.name, "");
        assert_eq!(manifest.version, "");
        assert!(manifest.capabilities.is_empty());
        assert!(manifest.dependencies.is_empty());
        assert!(manifest.firmware.is_empty());
        assert_eq!(manifest.package_uri, None);
        assert_eq!(manifest.signature, None);
        assert!(manifest.removable);
    }

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
    fn install_resolves_dependencies() {
        let mut manager = DriverManager::new();

        let mut bluetooth = DriverManifest::new("bluetooth", "bluetooth", "1.0.0");
        bluetooth.capabilities.push(Capability::Bluetooth);
        bluetooth
            .dependencies
            .push("bluetooth-firmware".to_string());
        let digest = bluetooth.signed_digest().expect("digest");
        bluetooth.signature = Some(PackageSignature {
            key_id: "test-key".to_string(),
            digest_sha256: digest,
        });
        let firmware = signed_manifest("bluetooth-firmware", Capability::Storage);

        manager.register_driver(bluetooth).unwrap();
        manager.register_driver(firmware).unwrap();

        manager.install_driver("bluetooth").unwrap();
        assert_eq!(manager.state("bluetooth"), Some(DriverState::Installed));
        assert_eq!(
            manager.state("bluetooth-firmware"),
            Some(DriverState::Installed)
        );
    }

    #[test]
    fn acquire_and_release_capability() {
        let mut manager = DriverManager::new();
        manager
            .register_driver(signed_manifest("bluetooth", Capability::Bluetooth))
            .unwrap();

        let driver_id = manager.acquire_capability(Capability::Bluetooth).unwrap();
        assert_eq!(driver_id, "bluetooth");
        assert_eq!(manager.state("bluetooth"), Some(DriverState::Enabled));
        assert_eq!(manager.active_leases("bluetooth"), Some(1));

        manager.release_capability("bluetooth").unwrap();
        assert_eq!(manager.state("bluetooth"), Some(DriverState::Disabled));
        assert_eq!(manager.active_leases("bluetooth"), Some(0));
    }

    #[test]
    fn remove_is_blocked_by_dependents() {
        let mut manager = DriverManager::new();

        let bluetooth = signed_manifest("bluetooth", Capability::Bluetooth);
        let mut settings = signed_manifest("settings", Capability::Storage);
        settings.dependencies.push("bluetooth".to_string());

        manager.register_driver(bluetooth).unwrap();
        manager.register_driver(settings).unwrap();

        let err = manager.remove_driver("bluetooth").unwrap_err();
        assert!(matches!(err, DriverError::DependencyInUse { .. }));
    }

    #[test]
    fn lease_releases_on_drop() {
        let mut manager = DriverManager::new();
        manager
            .register_driver(signed_manifest("bluetooth", Capability::Bluetooth))
            .unwrap();

        let broker = CapabilityBroker::new(manager);
        {
            let lease = broker.acquire(Capability::Bluetooth).unwrap();
            assert_eq!(lease.driver_id(), "bluetooth");
            assert_eq!(lease.capability(), &Capability::Bluetooth);
        }

        let manager = broker.manager.lock().unwrap();
        assert_eq!(manager.state("bluetooth"), Some(DriverState::Disabled));
        assert_eq!(manager.active_leases("bluetooth"), Some(0));
    }

    #[test]
    fn test_signed_digest_empty_and_edge_cases() {
        let mut manifest = DriverManifest::new("", "", "");
        let digest1 = manifest
            .signed_digest()
            .expect("Should produce digest for empty fields");

        manifest.signature = Some(PackageSignature {
            key_id: "test-key".to_string(),
            digest_sha256: "fake-digest".to_string(),
        });

        let digest2 = manifest
            .signed_digest()
            .expect("Should produce digest with signature present");
        assert_eq!(
            digest1, digest2,
            "Signature field should be ignored in digest calculation"
        );
    }
}
