use jiff::Timestamp;
use serde::Deserialize;

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

// TODO: Is making this an enum too strict? What if a new type is added? Use a
// string instead?
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct Version {
    pub id: String,
    pub r#type: Type,
    pub url: String,
    pub time: Timestamp,
    #[serde(rename = "releaseTime")]
    pub release_time: Timestamp,
    pub sha1: String,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: u8,
}

/// This matches version manifest v2. https://minecraft.fandom.com/wiki/Version_manifest.json
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<Version>,
}

impl VersionManifest {
    pub fn version(&self, version: impl AsRef<str>) -> Option<Version> {
        self.versions
            .iter()
            .find(|v| v.id == version.as_ref())
            .cloned()
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct Download {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct Downloads {
    pub server: Download,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct VersionMetadata {
    pub downloads: Downloads,
}
