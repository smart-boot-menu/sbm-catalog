use crate::catalog::{Catalog, arch_name, platform_name, strategy_name};
use crate::error::{Error, Result};
use sbm_types::{CatalogArtifact, CpuArch, FirmwarePlatform, OsCatalogEntry, OsReleaseEntry};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct Selection {
    pub arch: Option<CpuArch>,
    pub platform: Option<FirmwarePlatform>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedImage {
    pub entry_id: String,
    pub name: String,
    pub version: String,
    pub channel: String,
    pub arch: String,
    pub platforms: Vec<String>,
    pub install_strategy: String,
    pub filename: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub mirrors: Vec<String>,
    pub user_supplied_only: bool,
    pub apple_silicon: String,
    pub unattended: Vec<String>,
    pub limitations: String,
}

pub fn search(catalog: &Catalog, query: &str, selection: &Selection) -> Vec<ResolvedImage> {
    let needle = query.to_ascii_lowercase();
    let mut found = Vec::new();
    for loaded in &catalog.entries {
        let entry = &loaded.entry;
        let haystack = format!("{} {} {}", entry.id, entry.name, entry.vendor).to_ascii_lowercase();
        if !needle.is_empty() && !haystack.contains(&needle) {
            continue;
        }
        found.extend(images_for(
            entry,
            &loaded.profile.apple_silicon,
            &loaded.profile,
            selection,
        ));
    }
    found
}

pub fn resolve(catalog: &Catalog, query: &str, selection: &Selection) -> Result<ResolvedImage> {
    let matches = search(catalog, query, selection);
    let exact: Vec<_> = matches
        .iter()
        .filter(|image| image.entry_id.eq_ignore_ascii_case(query))
        .cloned()
        .collect();
    let candidates = if exact.is_empty() { matches } else { exact };
    match candidates.len() {
        0 => Err(Error::msg(no_match_message(catalog, query, selection))),
        1 => Ok(candidates.into_iter().next().expect("len checked")),
        _ => Err(Error::msg(format!(
            "query {query} matches {} images; pass --arch, --platform, or --version",
            candidates.len()
        ))),
    }
}

fn images_for(
    entry: &OsCatalogEntry,
    apple_silicon: &str,
    profile: &crate::compat::CompatibilityProfile,
    selection: &Selection,
) -> Vec<ResolvedImage> {
    let mut images = Vec::new();
    for release in &entry.releases {
        if let Some(version) = &selection.version
            && &release.version != version
        {
            continue;
        }
        if let Some(platform) = selection.platform
            && !release.supported_platforms.contains(&platform)
        {
            continue;
        }
        if let Some(arch) = selection.arch
            && !release.architectures.contains(&arch)
        {
            continue;
        }
        for artifact in &release.artifacts {
            let Some(profile_artifact) = profile.artifact(&artifact.filename) else {
                continue;
            };
            if let Some(arch) = selection.arch
                && !profile_artifact
                    .architectures
                    .iter()
                    .any(|name| name == &arch_name(arch))
            {
                continue;
            }
            let arch = selection
                .arch
                .map(arch_name)
                .or_else(|| profile_artifact.architectures.first().cloned())
                .unwrap_or_default();
            images.push(image_from(
                entry,
                release,
                artifact,
                &arch,
                apple_silicon,
                profile,
            ));
        }
    }
    images
}

fn image_from(
    entry: &OsCatalogEntry,
    release: &OsReleaseEntry,
    artifact: &CatalogArtifact,
    arch: &str,
    apple_silicon: &str,
    profile: &crate::compat::CompatibilityProfile,
) -> ResolvedImage {
    ResolvedImage {
        entry_id: entry.id.clone(),
        name: entry.name.clone(),
        version: release.version.clone(),
        channel: serde_json::to_value(release.channel)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_default(),
        arch: arch.to_string(),
        platforms: release
            .supported_platforms
            .iter()
            .copied()
            .map(platform_name)
            .collect(),
        install_strategy: strategy_name(release.install_strategy),
        filename: artifact.filename.clone(),
        size_bytes: artifact.size_bytes,
        sha256: artifact.sha256.clone(),
        mirrors: artifact.mirrors.clone(),
        user_supplied_only: artifact.user_supplied_only,
        apple_silicon: apple_silicon.to_string(),
        unattended: profile
            .automated_install
            .mechanisms
            .iter()
            .map(|item| format!("{}:{}", item.id, item.stability))
            .collect(),
        limitations: profile.limitations.clone(),
    }
}

fn no_match_message(catalog: &Catalog, query: &str, selection: &Selection) -> String {
    let mut message = format!("no catalog image matches {query}");
    if selection.platform == Some(FirmwarePlatform::AppleSiliconM1n1Uboot) {
        let generic_arm = search(
            catalog,
            query,
            &Selection {
                arch: Some(CpuArch::Aarch64),
                platform: Some(FirmwarePlatform::UefiNative),
                version: selection.version.clone(),
            },
        );
        if !generic_arm.is_empty() {
            message.push_str(
                "; aarch64 images exist for generic UEFI and are not offered on Apple Silicon",
            );
        }
    }
    message
}

pub fn list_lines(images: &[ResolvedImage]) -> String {
    let mut lines = Vec::new();
    for image in images {
        lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            image.entry_id,
            image.version,
            image.arch,
            image.platforms.join(","),
            image.filename,
            image.sha256
        ));
    }
    lines.join("\n")
}
