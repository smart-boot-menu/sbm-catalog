use crate::error::Result;
use crate::query::ResolvedImage;
use sbm_types::{
    BootRegistrationPlan, DiskBus, DiskIdentity, InstallStrategy, OsInstallPlan, SCHEMA_VERSION_V1,
    StorageOperationPlan, VerifiedArtifactRef,
};

const UNBOUND_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Build a machine-readable install plan that does not name a host disk and does not write one.
pub fn installer_plan(image: &ResolvedImage, source: &str) -> Result<OsInstallPlan> {
    let arch = crate::catalog::parse_arch(&image.arch).ok_or_else(|| {
        crate::error::Error::msg(format!(
            "cannot build a plan for architecture {}",
            image.arch
        ))
    })?;
    let strategy = parse_strategy(&image.install_strategy)?;
    let summary = vec![
        format!(
            "Boot the verified {} {} image {}.",
            image.name, image.version, image.filename
        ),
        "This plan does not partition, format, or write a physical disk.".to_string(),
        "sbm-core must replace stable_id unbound with a live DiskIdentity before any storage operation.".to_string(),
        format!("Automated installation ({}) is not executed by the catalog.", image.install_strategy),
    ];
    Ok(OsInstallPlan {
        schema_version: SCHEMA_VERSION_V1.to_string(),
        plan_id: format!(
            "sbm-catalog:{}:{}:{}",
            image.entry_id, image.version, image.arch
        ),
        catalog_entry_id: image.entry_id.clone(),
        catalog_release_version: image.version.clone(),
        target_arch: arch,
        strategy,
        artifact_verification: vec![VerifiedArtifactRef {
            filename: image.filename.clone(),
            size_bytes: image.size_bytes,
            sha256: image.sha256.clone(),
            source_url_or_path: source.to_string(),
        }],
        storage_plan: unbound_storage_plan(&image.entry_id, summary),
        boot_registration: BootRegistrationPlan {
            register_bls_or_efi: false,
            set_as_default: false,
            display_name: Some(format!("{} installer", image.name)),
        },
        offline_media_only: source.starts_with('/') || source.starts_with("file:"),
    })
}

fn unbound_storage_plan(entry_id: &str, dry_run_summary: Vec<String>) -> StorageOperationPlan {
    StorageOperationPlan {
        schema_version: SCHEMA_VERSION_V1.to_string(),
        plan_id: format!("sbm-catalog-storage:{entry_id}:unbound"),
        target_disk: DiskIdentity {
            stable_id: "unbound".to_string(),
            gpt_disk_guid: None,
            wwn_or_eui64: None,
            model: "unbound".to_string(),
            serial_hash: None,
            size_bytes: 0,
            logical_sector_size: 512,
            bus: DiskBus::Unknown,
            removable: false,
            read_only: true,
        },
        expected_partition_table_sha256: UNBOUND_SHA256.to_string(),
        operations: Vec::new(),
        destructive: false,
        reversible: true,
        touched_protected_classes: Vec::new(),
        dry_run_summary,
    }
}

fn parse_strategy(value: &str) -> Result<InstallStrategy> {
    match value {
        "direct_installer_boot" => Ok(InstallStrategy::DirectInstallerBoot),
        "rootfs_image_extract" => Ok(InstallStrategy::RootfsImageExtract),
        "bootc_oci" => Ok(InstallStrategy::BootcOci),
        "windows_wim_apply" => Ok(InstallStrategy::WindowsWimApply),
        "bsd_image" => Ok(InstallStrategy::BsdImage),
        _ => Err(crate::error::Error::msg(format!(
            "unknown install strategy {value}"
        ))),
    }
}

pub fn assert_plan_does_not_write(plan: &OsInstallPlan) -> Result<()> {
    if plan.storage_plan.destructive || !plan.storage_plan.operations.is_empty() {
        return Err(crate::error::Error::msg(
            "catalog plans must not contain storage write operations",
        ));
    }
    if plan.storage_plan.target_disk.stable_id != "unbound" {
        return Err(crate::error::Error::msg(
            "catalog plans must leave the target disk unbound",
        ));
    }
    Ok(())
}
