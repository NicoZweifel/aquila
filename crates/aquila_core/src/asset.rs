use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The "Manifest" is the source of truth for a game version.
/// It maps file paths ("textures/test.png") to content hashes ("x1b2c3...").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    /// The Version ID e.g., "v1.0" or P4 Changelist "1205" or Git SHA "a8f3b".
    pub version: String,

    /// Standard UTC timestamp.
    pub published_at: DateTime<Utc>,

    /// Who triggered the build.
    pub published_by: String,

    /// - Key: Game Path e.g., "assets/textures/test.png"
    /// - Value: Metadata
    pub assets: HashMap<String, AssetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    /// The SHA256 hash. This is the filename in the blob storage.
    pub hash: String,

    /// Size in bytes
    pub size: u64,

    /// Optional: Media Type
    pub mime_type: Option<String>,
}
