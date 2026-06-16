use rv8_browser_optimizations::driver_manager::{Capability, DriverManifest, PackageSignature};

pub fn signed_manifest(id: &str, capability: Capability) -> DriverManifest {
    let mut manifest = DriverManifest::new(id, id, "1.0.0");
    manifest.capabilities.push(capability);
    let digest = manifest.signed_digest().expect("digest");
    manifest.signature = Some(PackageSignature {
        key_id: "test-key".to_string(),
        digest_sha256: digest,
    });
    manifest
}
