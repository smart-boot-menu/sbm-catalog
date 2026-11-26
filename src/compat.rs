use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Catalog-local description that the v1 `OsCatalogEntry` wire type does not carry.
///
/// This is not a fork of `OsCatalogEntry`. `sbm-core` keeps using the canonical
/// entry for download and install-plan fields. These profiles add licensing,
/// per-image architecture, boot method, and unattended-mechanism metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityProfile {
    pub schema_version: String,
    pub catalog_entry_id: String,
    pub license: String,
    /// `unsupported` or `explicit-m1n1-uboot-artifact`.
    /// Generic `aarch64` images must stay `unsupported`.
    pub apple_silicon: String,
    pub automated_install: AutomatedInstall,
    pub artifacts: Vec<ArtifactProfile>,
    pub limitations: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomatedInstall {
    /// Named upstream mechanisms. None of them are executed by this catalog.
    pub mechanisms: Vec<UnattendedMechanism>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnattendedMechanism {
    pub id: String,
    /// `stable`, `experimental`, or `unsupported`.
    pub stability: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactProfile {
    pub filename: String,
    pub architectures: Vec<String>,
    pub image_format: String,
    pub boot_methods: Vec<String>,
    #[serde(default)]
    pub checksum_urls: Vec<String>,
    #[serde(default)]
    pub signature_urls: Vec<String>,
    /// `signed-metadata`, `signed-image`, `unverified-signature`, `checksum-only`, or `user-supplied`.
    ///
    /// `signed-metadata` means a pinned key verified a checksum document.
    /// A hash stored next to an image is `checksum-only` until that happens.
    pub authenticity: String,
    /// File name under `keys/upstream/`. Empty when no key is pinned.
    #[serde(default)]
    pub pinned_key: String,
}

impl ArtifactProfile {
    pub fn automatic_download(&self) -> bool {
        matches!(
            self.authenticity.as_str(),
            "signed-metadata" | "signed-image"
        )
    }
}

impl CompatibilityProfile {
    pub fn artifact(&self, filename: &str) -> Option<&ArtifactProfile> {
        self.artifacts.iter().find(|item| item.filename == filename)
    }

    pub fn arch_map(&self) -> BTreeMap<String, Vec<String>> {
        self.artifacts
            .iter()
            .map(|item| (item.filename.clone(), item.architectures.clone()))
            .collect()
    }
}
