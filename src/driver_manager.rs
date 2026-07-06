// ponytail: stub — replace with rv8 types when rv8 linkage lands
// Another agent handles real rv8 integration.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    Render(u32),
    Network(u32),
    Storage(u32),
    Compute(u32),
    Media(u32),
}

#[derive(Debug, Clone)]
pub struct PackageSignature {
    pub key_id: String,
    pub digest_sha256: String,
}

#[derive(Debug, Clone)]
pub struct DriverManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<Capability>,
    pub signature: Option<PackageSignature>,
}

impl DriverManifest {
    pub fn new(id: &str, name: &str, version: &str) -> Self {
        DriverManifest {
            id: id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            capabilities: Vec::new(),
            signature: None,
        }
    }

    pub fn signed_digest(&self) -> Result<String, String> {
        Ok(format!("{}-{}-{}", self.id, self.name, self.version))
    }
}
