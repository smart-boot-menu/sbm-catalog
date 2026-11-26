use crate::compat::CompatibilityProfile;
use crate::error::{Error, Result, io_at};
use sbm_types::{CpuArch, FirmwarePlatform, InstallStrategy, OsCatalogEntry, OsFamily};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const FAMILIES: &[&str] = &["linux", "bsd", "windows", "recovery"];
const UNPINNED_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug)]
pub struct Catalog {
    pub root: PathBuf,
    pub entries: Vec<LoadedEntry>,
}

#[derive(Debug)]
pub struct LoadedEntry {
    pub path: PathBuf,
    pub entry: OsCatalogEntry,
    pub profile_path: PathBuf,
    pub profile: CompatibilityProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub path: String,
    pub message: String,
}

impl Catalog {
    pub fn load(root: &Path) -> Result<Self> {
        let mut entries = Vec::new();
        for family in FAMILIES {
            let dir = root.join(family);
            if !dir.is_dir() {
                return Err(Error::msg(format!(
                    "missing catalog directory {}",
                    dir.display()
                )));
            }
            let mut paths = fs::read_dir(&dir)
                .map_err(|err| io_at(&dir, err))?
                .map(|item| {
                    item.map(|entry| entry.path())
                        .map_err(|err| io_at(&dir, err))
                })
                .collect::<Result<Vec<_>>>()?;
            paths.sort();
            for path in paths {
                if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                    continue;
                }
                let text = fs::read_to_string(&path).map_err(|err| io_at(&path, err))?;
                reject_recipe_text(&path, &text)?;
                let entry: OsCatalogEntry =
                    serde_json::from_str(&text).map_err(|source| Error::Json {
                        path: path.clone(),
                        source,
                    })?;
                let profile_path = root
                    .join("compatibility")
                    .join(format!("{}.json", entry.id));
                let profile_text =
                    fs::read_to_string(&profile_path).map_err(|err| io_at(&profile_path, err))?;
                reject_recipe_text(&profile_path, &profile_text)?;
                let profile: CompatibilityProfile =
                    serde_json::from_str(&profile_text).map_err(|source| Error::Json {
                        path: profile_path.clone(),
                        source,
                    })?;
                entries.push(LoadedEntry {
                    path,
                    entry,
                    profile_path,
                    profile,
                });
            }
        }
        entries.sort_by(|left, right| left.entry.id.cmp(&right.entry.id));
        Ok(Self {
            root: root.to_path_buf(),
            entries,
        })
    }

    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = Vec::new();
        scan_tree(&self.root, &mut issues);
        let mut ids = BTreeSet::new();
        let mut filenames = BTreeSet::new();
        for loaded in &self.entries {
            let label = loaded.path.display().to_string();
            if !ids.insert(loaded.entry.id.clone()) {
                issues.push(issue(
                    &label,
                    format!("duplicate catalog id {}", loaded.entry.id),
                ));
            }
            let stem = loaded
                .path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("");
            if stem != loaded.entry.id {
                issues.push(issue(
                    &label,
                    format!(
                        "file name {stem}.json does not match id {}",
                        loaded.entry.id
                    ),
                ));
            }
            validate_entry(
                &loaded.entry,
                &loaded.profile,
                &label,
                &mut filenames,
                &mut issues,
            );
        }
        issues
    }

    pub fn get(&self, id: &str) -> Option<&LoadedEntry> {
        self.entries.iter().find(|loaded| loaded.entry.id == id)
    }
}

fn validate_entry(
    entry: &OsCatalogEntry,
    profile: &CompatibilityProfile,
    label: &str,
    filenames: &mut BTreeSet<String>,
    issues: &mut Vec<Issue>,
) {
    if entry.schema_version != sbm_types::SCHEMA_VERSION_V1 {
        issues.push(issue(label, "schema_version must be 1.0"));
    }
    if !valid_id(&entry.id) {
        issues.push(issue(label, "id must be a lowercase slug"));
    }
    if entry.name.trim().is_empty() || entry.vendor.trim().is_empty() {
        issues.push(issue(label, "name and vendor are required"));
    }
    if !entry.homepage_url.starts_with("https://") {
        issues.push(issue(label, "homepage_url must be https"));
    }
    if entry.releases.is_empty() {
        issues.push(issue(label, "at least one release is required"));
    }
    if profile.schema_version != "1.0" {
        issues.push(issue(label, "compatibility schema_version must be 1.0"));
    }
    if profile.catalog_entry_id != entry.id {
        issues.push(issue(
            label,
            "compatibility catalog_entry_id does not match the entry",
        ));
    }
    if profile.license.trim().is_empty() || profile.limitations.trim().is_empty() {
        issues.push(issue(label, "license and limitations are required"));
    }
    match profile.apple_silicon.as_str() {
        "unsupported" | "explicit-m1n1-uboot-artifact" => {}
        other => issues.push(issue(
            label,
            format!("apple_silicon value {other} is not recognized"),
        )),
    }
    for mechanism in &profile.automated_install.mechanisms {
        if !matches!(
            mechanism.stability.as_str(),
            "stable" | "experimental" | "unsupported"
        ) {
            issues.push(issue(
                label,
                format!(
                    "unattended stability {} is not recognized",
                    mechanism.stability
                ),
            ));
        }
        if mechanism.id.contains('/') || mechanism.id.contains(' ') {
            issues.push(issue(
                label,
                "unattended mechanism id must be a single token",
            ));
        }
    }

    let mut seen_arch_versions = HashSet::new();
    for release in &entry.releases {
        if release.version.is_empty() || release.recipe_version.is_empty() {
            issues.push(issue(label, "version and recipe_version are required"));
        }
        if release.architectures.is_empty() || release.supported_platforms.is_empty() {
            issues.push(issue(
                label,
                format!(
                    "release {} needs architectures and platforms",
                    release.version
                ),
            ));
        }
        if let Some(eol) = &release.eol_date
            && !is_iso_date(eol)
        {
            issues.push(issue(label, format!("eol_date {eol} is not YYYY-MM-DD")));
        }
        let unique_arches: HashSet<_> = release.architectures.iter().copied().collect();
        if unique_arches.len() != release.architectures.len() {
            issues.push(issue(
                label,
                format!("release {} repeats an architecture", release.version),
            ));
        }
        for arch in &release.architectures {
            if !seen_arch_versions.insert((release.version.clone(), *arch)) {
                issues.push(issue(
                    label,
                    format!(
                        "duplicate release {} for {}",
                        release.version,
                        arch_name(*arch)
                    ),
                ));
            }
        }
        if release
            .supported_platforms
            .contains(&FirmwarePlatform::AppleSiliconM1n1Uboot)
            && profile.apple_silicon != "explicit-m1n1-uboot-artifact"
        {
            issues.push(issue(
                label,
                "apple-silicon-m1n1-uboot requires apple_silicon = explicit-m1n1-uboot-artifact; aarch64 does not imply Apple Silicon",
            ));
        }
        if profile.apple_silicon == "explicit-m1n1-uboot-artifact"
            && (!release.architectures.contains(&CpuArch::Aarch64)
                || !release
                    .supported_platforms
                    .contains(&FirmwarePlatform::AppleSiliconM1n1Uboot))
        {
            issues.push(issue(
                label,
                "explicit Apple Silicon entries must list aarch64 and apple-silicon-m1n1-uboot",
            ));
        }
        if !strategy_matches(entry.os_family, release.install_strategy) {
            issues.push(issue(
                label,
                format!(
                    "install strategy {} is not valid for {}",
                    strategy_name(release.install_strategy),
                    family_name(entry.os_family)
                ),
            ));
        }
        if release.artifacts.is_empty() {
            issues.push(issue(
                label,
                format!("release {} has no artifacts", release.version),
            ));
        }
        let mut covered = HashSet::new();
        for artifact in &release.artifacts {
            if !filenames.insert(artifact.filename.clone()) {
                issues.push(issue(
                    label,
                    format!("duplicate artifact filename {}", artifact.filename),
                ));
            }
            if artifact.filename.contains('/')
                || artifact.filename.contains("..")
                || artifact.filename.is_empty()
            {
                issues.push(issue(
                    label,
                    format!("unsafe filename {}", artifact.filename),
                ));
            }
            let sha_ok = is_hex(&artifact.sha256, 64);
            if !sha_ok {
                issues.push(issue(
                    label,
                    format!(
                        "{} sha256 must be 64 lowercase hex characters",
                        artifact.filename
                    ),
                ));
            }
            if let Some(sha512) = &artifact.sha512
                && !is_hex(sha512, 128)
            {
                issues.push(issue(
                    label,
                    format!(
                        "{} sha512 must be 128 lowercase hex characters",
                        artifact.filename
                    ),
                ));
            }
            if let Some(url) = &artifact.upstream_signature_url
                && https_problem(url).is_some()
            {
                issues.push(issue(label, format!("signature URL {url} must be https")));
            }
            let profile_artifact = profile.artifact(&artifact.filename);
            if profile_artifact.is_none() {
                issues.push(issue(
                    label,
                    format!(
                        "compatibility profile is missing artifact {}",
                        artifact.filename
                    ),
                ));
            }
            if artifact.user_supplied_only {
                if !artifact.mirrors.is_empty() {
                    issues.push(issue(
                        label,
                        format!(
                            "{} is user supplied and must not list download mirrors",
                            artifact.filename
                        ),
                    ));
                }
                let unpinned = artifact.sha256 == UNPINNED_SHA256 && artifact.size_bytes == 0;
                let pinned =
                    sha_ok && artifact.sha256 != UNPINNED_SHA256 && artifact.size_bytes > 0;
                if !unpinned && !pinned {
                    issues.push(issue(
                        label,
                        format!("{} user-supplied integrity must be unpinned zeros or a real hash and size", artifact.filename),
                    ));
                }
            } else {
                if artifact.sha256 == UNPINNED_SHA256 || artifact.size_bytes == 0 {
                    issues.push(issue(
                        label,
                        format!("{} is missing integrity metadata", artifact.filename),
                    ));
                }
                if artifact.mirrors.is_empty() {
                    issues.push(issue(
                        label,
                        format!("{} needs at least one https mirror", artifact.filename),
                    ));
                }
                for mirror in &artifact.mirrors {
                    if let Some(problem) = https_problem(mirror) {
                        issues.push(issue(label, format!("{mirror}: {problem}")));
                    }
                }
            }
            if let Some(profile_artifact) = profile_artifact {
                if profile_artifact.image_format.is_empty()
                    || profile_artifact.boot_methods.is_empty()
                {
                    issues.push(issue(
                        label,
                        format!(
                            "{} needs an image format and boot method",
                            artifact.filename
                        ),
                    ));
                }
                for arch_name in &profile_artifact.architectures {
                    match parse_arch(arch_name) {
                        Some(arch) => {
                            covered.insert(arch);
                            if !release.architectures.contains(&arch) {
                                issues.push(issue(
                                    label,
                                    format!(
                                        "{} lists {arch_name}, which the release does not support",
                                        artifact.filename
                                    ),
                                ));
                            }
                        }
                        None => issues.push(issue(
                            label,
                            format!("unsupported architecture {arch_name}"),
                        )),
                    }
                }
            }
        }
        for arch in &release.architectures {
            if !covered.contains(arch) {
                issues.push(issue(
                    label,
                    format!(
                        "release {} has no artifact for {}",
                        release.version,
                        arch_name(*arch)
                    ),
                ));
            }
        }
    }
}

fn scan_tree(root: &Path, issues: &mut Vec<Issue>) {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) => {
                issues.push(issue(dir.display().to_string(), err.to_string()));
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
            if name != ".gitkeep" && ext != "json" {
                issues.push(issue(
                    path.display().to_string(),
                    "catalog tree contains a non-declarative file",
                ));
            }
            if let Ok(bytes) = fs::read(&path)
                && (bytes.starts_with(b"#!")
                    || bytes
                        .windows(22)
                        .any(|window| window == b"-----BEGIN PRIVATE KEY"))
            {
                issues.push(issue(
                    path.display().to_string(),
                    "catalog tree contains executable or private-key material",
                ));
            }
        }
    }
}

fn reject_recipe_text(path: &Path, text: &str) -> Result<()> {
    let lowered = text.to_ascii_lowercase();
    for needle in [
        "#!/bin/",
        "#!/usr/bin/",
        "curl |",
        "wget |",
        "begin private key",
    ] {
        if lowered.contains(needle) {
            return Err(Error::msg(format!(
                "{} contains forbidden recipe text {needle}",
                path.display()
            )));
        }
    }
    Ok(())
}

pub fn strategy_matches(family: OsFamily, strategy: InstallStrategy) -> bool {
    match strategy {
        InstallStrategy::DirectInstallerBoot => true,
        InstallStrategy::RootfsImageExtract | InstallStrategy::BootcOci => {
            family == OsFamily::Linux
        }
        InstallStrategy::WindowsWimApply => family == OsFamily::Windows,
        InstallStrategy::BsdImage => family == OsFamily::Bsd,
    }
}

pub fn https_problem(url: &str) -> Option<&'static str> {
    let Ok(parsed) = url::Url::parse(url) else {
        return Some("invalid URL");
    };
    if parsed.scheme() != "https" {
        return Some("only https mirrors are accepted");
    }
    if parsed.username() != "" || parsed.password().is_some() {
        return Some("credentials in URLs are not accepted");
    }
    if parsed.host_str().is_none() {
        return Some("URL has no host");
    }
    None
}

pub fn artifact_url(mirror: &str, filename: &str) -> String {
    if mirror.ends_with('/') {
        format!("{mirror}{filename}")
    } else if mirror.rsplit('/').next() == Some(filename) {
        mirror.to_string()
    } else {
        format!("{mirror}/{filename}")
    }
}

pub fn is_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn issue(path: impl Into<String>, message: impl Into<String>) -> Issue {
    Issue {
        path: path.into(),
        message: message.into(),
    }
}

pub fn arch_name(arch: CpuArch) -> String {
    json_enum(arch)
}

pub fn platform_name(platform: FirmwarePlatform) -> String {
    json_enum(platform)
}

pub fn strategy_name(strategy: InstallStrategy) -> String {
    json_enum(strategy)
}

pub fn family_name(family: OsFamily) -> String {
    json_enum(family)
}

fn json_enum<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn parse_arch(value: &str) -> Option<CpuArch> {
    match value {
        "x86_64" => Some(CpuArch::X86_64),
        "aarch64" => Some(CpuArch::Aarch64),
        "ia32" => Some(CpuArch::Ia32),
        "riscv64" => Some(CpuArch::Riscv64),
        _ => None,
    }
}

pub fn parse_platform(value: &str) -> Option<FirmwarePlatform> {
    match value {
        "uefi-native" => Some(FirmwarePlatform::UefiNative),
        "uefi-uboot" => Some(FirmwarePlatform::UefiUboot),
        "apple-silicon-m1n1-uboot" => Some(FirmwarePlatform::AppleSiliconM1n1Uboot),
        "legacy-bios" => Some(FirmwarePlatform::LegacyBios),
        _ => None,
    }
}

pub fn export_offline(
    catalog_root: &Path,
    metadata: Option<&Path>,
    destination: &Path,
) -> Result<()> {
    if destination.exists() {
        return Err(Error::msg(format!(
            "{} already exists",
            destination.display()
        )));
    }
    copy_dir(catalog_root, &destination.join("catalog"))?;
    if let Some(metadata) = metadata {
        copy_dir(metadata, &destination.join("metadata"))?;
    }
    fs::write(
        destination.join("OFFLINE.txt"),
        "Offline Smart Boot Menu catalog export. Image files are not included.\nValidate with: sbm-catalog validate --catalog catalog\n",
    )?;
    Ok(())
}

fn copy_dir(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).map_err(|err| io_at(destination, err))?;
    for entry in fs::read_dir(source).map_err(|err| io_at(source, err))? {
        let entry = entry.map_err(|err| io_at(source, err))?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|err| io_at(&from, err))?;
        }
    }
    Ok(())
}

pub fn index_document(
    catalog: &Catalog,
    revision: u64,
    published_at_unix: i64,
) -> sbm_types::CatalogIndex {
    sbm_types::CatalogIndex {
        schema_version: sbm_types::SCHEMA_VERSION_V1.to_string(),
        catalog_revision: revision,
        published_at_unix,
        entries: catalog
            .entries
            .iter()
            .map(|loaded| loaded.entry.clone())
            .collect(),
    }
}

pub fn profile_by_id(catalog: &Catalog) -> BTreeMap<String, &CompatibilityProfile> {
    catalog
        .entries
        .iter()
        .map(|loaded| (loaded.entry.id.clone(), &loaded.profile))
        .collect()
}
