use jiff::SignedDuration;
use sbm_catalog::catalog::{Catalog, export_offline};
use sbm_catalog::download::{DownloadRequest, download, import_user_image, sha256_file};
use sbm_catalog::plan::{assert_plan_does_not_write, installer_plan};
use sbm_catalog::query::{Selection, resolve, search};
use sbm_catalog::tuf::{PublishRequest, publish, verify_repository};
use sbm_catalog::upstream::{
    gpg_program, signed_text_contains_sha256, text_contains_sha512, verify_cleartext,
    verify_detached,
};
use sbm_types::{CpuArch, FirmwarePlatform};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::thread;

fn manifest_catalog() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("catalog")
}

fn selection(arch: Option<CpuArch>, platform: Option<FirmwarePlatform>) -> Selection {
    Selection {
        arch,
        platform,
        version: None,
    }
}

fn copy_catalog(destination: &Path) {
    fn copy_dir(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                copy_dir(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    copy_dir(&manifest_catalog(), destination);
}

#[test]
fn shipped_catalog_validates() {
    let catalog = Catalog::load(&manifest_catalog()).unwrap();
    let issues = catalog.validate();
    assert!(issues.is_empty(), "{issues:?}");
    assert!(catalog.entries.len() >= 12);
}

#[test]
fn architecture_filter_does_not_treat_arm_as_apple_silicon() {
    let catalog = Catalog::load(&manifest_catalog()).unwrap();
    let arm = resolve(
        &catalog,
        "debian",
        &selection(Some(CpuArch::Aarch64), Some(FirmwarePlatform::UefiNative)),
    )
    .unwrap();
    assert_eq!(arm.filename, "debian-13.8.0-arm64-netinst.iso");
    assert_eq!(arm.apple_silicon, "unsupported");
    assert!(
        !arm.platforms
            .iter()
            .any(|platform| platform.contains("apple"))
    );
    let apple = resolve(
        &catalog,
        "debian",
        &selection(
            Some(CpuArch::Aarch64),
            Some(FirmwarePlatform::AppleSiliconM1n1Uboot),
        ),
    );
    let message = apple.unwrap_err().to_string();
    assert!(
        message.contains("not offered on Apple Silicon"),
        "{message}"
    );
}

#[test]
fn ambiguous_ubuntu_requires_an_architecture() {
    let catalog = Catalog::load(&manifest_catalog()).unwrap();
    let error = resolve(&catalog, "ubuntu", &selection(None, None)).unwrap_err();
    assert!(error.to_string().contains("matches"), "{error}");
    let amd64 = resolve(&catalog, "ubuntu", &selection(Some(CpuArch::X86_64), None)).unwrap();
    assert!(
        amd64
            .platforms
            .iter()
            .any(|platform| platform == "legacy-bios")
    );
    let arm = resolve(&catalog, "ubuntu", &selection(Some(CpuArch::Aarch64), None)).unwrap();
    assert_eq!(arm.platforms, vec!["uefi-native".to_string()]);
}

#[test]
fn windows_download_is_refused_until_imported() {
    let catalog = Catalog::load(&manifest_catalog()).unwrap();
    let image = resolve(&catalog, "windows", &selection(Some(CpuArch::X86_64), None)).unwrap();
    assert!(image.user_supplied_only);
    assert!(image.mirrors.is_empty());
    let error = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(download(DownloadRequest {
            urls: &image.mirrors,
            filename: &image.filename,
            size_bytes: image.size_bytes,
            sha256: &image.sha256,
            cache: Path::new("build/cache"),
            allow_loopback_http: false,
            offline: false,
        }))
        .unwrap_err();
    assert!(error.to_string().contains("refusing"), "{error}");
}

#[test]
fn install_plan_does_not_name_a_disk() {
    let catalog = Catalog::load(&manifest_catalog()).unwrap();
    let image = resolve(
        &catalog,
        "debian",
        &selection(Some(CpuArch::Aarch64), Some(FirmwarePlatform::UefiNative)),
    )
    .unwrap();
    let plan = installer_plan(&image, &image.mirrors[0]).unwrap();
    assert_plan_does_not_write(&plan).unwrap();
    assert!(plan.storage_plan.operations.is_empty());
    assert!(!plan.storage_plan.destructive);
    assert_eq!(plan.storage_plan.target_disk.stable_id, "unbound");
}

#[test]
fn rejects_duplicate_ids_http_mirrors_bad_arches_and_scripts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("catalog");
    copy_catalog(&root);
    fs::write(root.join("linux/notes.txt"), "not declarative\n").unwrap();
    let mut issues = Catalog::load(&root).unwrap().validate();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("non-declarative")),
        "{issues:?}"
    );

    let ubuntu = root.join("linux/ubuntu.json");
    let mut text = fs::read_to_string(&ubuntu).unwrap();
    text = text.replace("https://releases.ubuntu.com", "http://releases.ubuntu.com");
    fs::write(&ubuntu, &text).unwrap();
    issues = Catalog::load(&root).unwrap().validate();
    assert!(
        issues.iter().any(|issue| issue.message.contains("https")),
        "{issues:?}"
    );

    let profile = root.join("compatibility/ubuntu.json");
    let mut profile_text = fs::read_to_string(&profile).unwrap();
    profile_text = profile_text.replace("\"x86_64\"", "\"powerpc\"");
    fs::write(&profile, profile_text).unwrap();
    issues = Catalog::load(&root).unwrap().validate();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("unsupported architecture")),
        "{issues:?}"
    );

    fs::write(root.join("linux/evil.json"), "#!/bin/sh\necho no\n").unwrap();
    let load = Catalog::load(&root);
    assert!(load.unwrap_err().to_string().contains("forbidden"));
}

#[test]
fn rejects_private_keys_and_apple_silicon_without_an_explicit_artifact() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("catalog");
    copy_catalog(&root);
    fs::write(
        root.join("linux/secret.json"),
        "-----BEGIN PRIVATE KEY-----\nAAAA\n",
    )
    .unwrap();
    assert!(
        Catalog::load(&root)
            .unwrap_err()
            .to_string()
            .contains("forbidden")
    );
    fs::remove_file(root.join("linux/secret.json")).unwrap();
    let debian = root.join("linux/debian.json");
    let text = fs::read_to_string(&debian)
        .unwrap()
        .replace("\"uefi-native\"", "\"apple-silicon-m1n1-uboot\"");
    fs::write(&debian, text).unwrap();
    let issues = Catalog::load(&root).unwrap().validate();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("explicit-m1n1-uboot-artifact")),
        "{issues:?}"
    );
}

#[test]
fn missing_integrity_is_rejected_for_redistributable_images() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("catalog");
    copy_catalog(&root);
    let path = root.join("linux/fedora.json");
    let text = fs::read_to_string(&path).unwrap().replace(
        "a1e916f139a05955b943cf183b020780cca9c399b02b8f5ec4354c9e5c1dff1a",
        &"0".repeat(64),
    );
    fs::write(&path, text).unwrap();
    let issues = Catalog::load(&root).unwrap().validate();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("integrity")),
        "{issues:?}"
    );
}

#[test]
fn search_lists_only_the_selected_architecture() {
    let catalog = Catalog::load(&manifest_catalog()).unwrap();
    let found = search(&catalog, "fedora", &selection(Some(CpuArch::Aarch64), None));
    assert_eq!(found.len(), 1);
    assert!(found[0].filename.contains("aarch64"));
}

#[test]
fn offline_export_copies_entries_without_images() {
    let tmp = tempfile::tempdir().unwrap();
    let destination = tmp.path().join("offline");
    export_offline(&manifest_catalog(), None, &destination).unwrap();
    let exported = Catalog::load(&destination.join("catalog")).unwrap();
    assert!(exported.validate().is_empty());
    assert!(destination.join("OFFLINE.txt").is_file());
    assert!(export_offline(&manifest_catalog(), None, &destination).is_err());
}

#[test]
fn import_records_a_local_image_without_copying_it_into_the_catalog() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("windows.iso");
    fs::write(&source, b"not-a-real-windows-image").unwrap();
    let record = import_user_image("windows", &source, tmp.path()).unwrap();
    assert_eq!(record.size_bytes, 24);
    assert_eq!(record.sha256, sha256_file(&source).unwrap());
    assert!(!manifest_catalog().join("windows/windows.iso").exists());
}

#[test]
fn checksum_parsers_match_upstream_layouts() {
    let fedora = "SHA256 (Fedora-Server-netinst-x86_64-45-1.4.iso) = a1e916f139a05955b943cf183b020780cca9c399b02b8f5ec4354c9e5c1dff1a\n";
    assert!(signed_text_contains_sha256(
        fedora,
        "Fedora-Server-netinst-x86_64-45-1.4.iso",
        "a1e916f139a05955b943cf183b020780cca9c399b02b8f5ec4354c9e5c1dff1a"
    ));
    let debian = "3770c7a54890f7574a68a8b402e553806a81d6bccfb379554caad3a9a857931e *debian-13.8.0-amd64-netinst.iso\n";
    assert!(signed_text_contains_sha256(
        debian,
        "debian-13.8.0-amd64-netinst.iso",
        "3770c7a54890f7574a68a8b402e553806a81d6bccfb379554caad3a9a857931e"
    ));
    assert!(text_contains_sha512(
        "dd3605a3be177497d71b998bb608d7a276717f0f6670f53a6047067c9551cc665f38430e8e72b6ad5f6e181f0f9294502209991da738d04547a1a32a68471f89  Leap-16.1-online-installer-x86_64.install.iso\n",
        "Leap-16.1-online-installer-x86_64.install.iso",
        "dd3605a3be177497d71b998bb608d7a276717f0f6670f53a6047067c9551cc665f38430e8e72b6ad5f6e181f0f9294502209991da738d04547a1a32a68471f89"
    ));
    assert!(!signed_text_contains_sha256(
        debian,
        "debian-13.8.0-amd64-netinst.iso",
        &"ab".repeat(32)
    ));
}

#[test]
fn pinned_signatures_reject_tampered_checksums() {
    if gpg_program().is_err() {
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let key = fs::read_to_string(root.join("keys/upstream/fedora-45.asc")).unwrap();
    let mut message =
        fs::read(root.join("tests/fixtures/fedora-45-server-x86_64.CHECKSUM")).unwrap();
    verify_cleartext(&key, &message).unwrap();
    let arm = fs::read(root.join("tests/fixtures/fedora-45-server-aarch64.CHECKSUM")).unwrap();
    verify_cleartext(&key, &arm).unwrap();
    message[80] ^= 0x01;
    assert!(verify_cleartext(&key, &message).is_err());
    let debian_key = fs::read_to_string(root.join("keys/upstream/debian-cd.asc")).unwrap();
    let sums = fs::read(root.join("tests/fixtures/debian-13.8.0-amd64-SHA256SUMS")).unwrap();
    let signature =
        fs::read(root.join("tests/fixtures/debian-13.8.0-amd64-SHA256SUMS.sign")).unwrap();
    verify_detached(&debian_key, &signature, &sums).unwrap();
    let mut tampered = sums.clone();
    tampered[0] ^= 0x01;
    assert!(verify_detached(&debian_key, &signature, &tampered).is_err());
}

#[tokio::test]
async fn resumable_download_checks_size_and_hash() {
    let body = b"catalog-download-test-bytes".to_vec();
    let tmp = tempfile::tempdir().unwrap();
    let full = tmp.path().join("full.iso");
    fs::write(&full, &body).unwrap();
    let sha = sha256_file(&full).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let served = body.clone();
    thread::spawn(move || {
        for _ in 0..4 {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buf = [0_u8; 2048];
            let read = socket.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..read]);
            let start = request
                .lines()
                .find(|line| line.to_ascii_lowercase().starts_with("range:"))
                .and_then(|line| line.split('=').nth(1))
                .and_then(|value| value.trim().trim_end_matches('-').parse::<usize>().ok())
                .unwrap_or(0);
            let payload = &served[start..];
            let status = if start == 0 {
                "200 OK"
            } else {
                "206 Partial Content"
            };
            let header = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload.len()
            );
            socket.write_all(header.as_bytes()).unwrap();
            socket.write_all(payload).unwrap();
        }
    });
    let cache = tmp.path();
    let filename = "sample.iso";
    fs::write(cache.join(filename), &body[..8]).unwrap();
    fs::write(
        cache.join(format!("{filename}.partial.json")),
        format!(
            "{{\"filename\":\"{filename}\",\"size_bytes\":{},\"sha256\":\"{sha}\"}}",
            body.len()
        ),
    )
    .unwrap();
    let url = format!("http://127.0.0.1:{port}/{filename}");
    let path = download(DownloadRequest {
        urls: &[url],
        filename,
        size_bytes: body.len() as u64,
        sha256: &sha,
        cache,
        allow_loopback_http: true,
        offline: false,
    })
    .await
    .unwrap();
    assert_eq!(fs::read(path).unwrap(), body);
}

#[tokio::test]
async fn development_tuf_rejects_tampering_rollback_and_expiry() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog = manifest_catalog();
    let keys = tmp.path().join("keys");
    let out = tmp.path().join("repo");
    let published = publish(PublishRequest::development(
        catalog.clone(),
        keys.clone(),
        out.clone(),
        NonZeroU64::new(1).unwrap(),
    ))
    .await
    .unwrap();
    let count = verify_repository(
        &published.metadata,
        &published.targets,
        &tmp.path().join("store-v1"),
        true,
    )
    .await
    .unwrap();
    assert!(count >= 24, "{count}");
    let saved_timestamp = fs::read(published.metadata.join("timestamp.json")).unwrap();
    let mut tampered = fs::read(published.metadata.join("targets.json")).unwrap();
    let flip = tampered.len() / 2;
    tampered[flip] ^= 0x1;
    fs::write(published.metadata.join("targets.json"), &tampered).unwrap();
    assert!(
        verify_repository(
            &published.metadata,
            &published.targets,
            &tmp.path().join("store-tamper"),
            true,
        )
        .await
        .is_err()
    );
    let republished = publish(PublishRequest::development(
        catalog.clone(),
        keys.clone(),
        out.clone(),
        NonZeroU64::new(2).unwrap(),
    ))
    .await
    .unwrap();
    verify_repository(
        &republished.metadata,
        &republished.targets,
        &tmp.path().join("store-v1"),
        true,
    )
    .await
    .unwrap();
    fs::write(republished.metadata.join("timestamp.json"), saved_timestamp).unwrap();
    assert!(
        verify_repository(
            &republished.metadata,
            &republished.targets,
            &tmp.path().join("store-v1"),
            true,
        )
        .await
        .is_err()
    );

    let expired_out = tmp.path().join("expired");
    let mut expired = PublishRequest::development(
        catalog,
        tmp.path().join("expired-keys"),
        expired_out.clone(),
        NonZeroU64::new(1).unwrap(),
    );
    expired.timestamp_lifetime = SignedDuration::from_secs(-3600);
    let expired_repo = publish(expired).await.unwrap();
    assert!(
        verify_repository(
            &expired_repo.metadata,
            &expired_repo.targets,
            &tmp.path().join("store-expired"),
            true,
        )
        .await
        .is_err()
    );
    verify_repository(
        &expired_repo.metadata,
        &expired_repo.targets,
        &tmp.path().join("store-unsafe"),
        false,
    )
    .await
    .unwrap();
}

#[test]
fn architecture_and_firmware_filters_drop_incompatible_images() {
    let catalog = Catalog::load(&manifest_catalog()).unwrap();
    let bios = search(
        &catalog,
        "",
        &selection(None, Some(FirmwarePlatform::LegacyBios)),
    );
    assert!(!bios.is_empty());
    assert!(bios.iter().all(|image| {
        image
            .platforms
            .iter()
            .any(|platform| platform == "legacy-bios")
    }));
    assert!(bios.iter().all(|image| image.entry_id != "fedora"));
    let arm = search(&catalog, "", &selection(Some(CpuArch::Aarch64), None));
    assert!(arm.iter().all(|image| image.arch == "aarch64"));
    for absent in [
        "arch-linux",
        "linux-mint",
        "gparted-live",
        "proxmox-ve",
        "systemrescue",
    ] {
        assert!(
            arm.iter().all(|image| image.entry_id != absent),
            "{absent} appeared in an aarch64 listing"
        );
    }
    let x86 = search(&catalog, "", &selection(Some(CpuArch::X86_64), None));
    assert!(x86.iter().any(|image| image.entry_id == "arch-linux"));
    assert!(x86.iter().all(|image| image.arch == "x86_64"));
}

#[test]
fn checksums_are_not_treated_as_upstream_authentication() {
    let catalog = Catalog::load(&manifest_catalog()).unwrap();
    let nix = resolve(&catalog, "nixos", &selection(Some(CpuArch::X86_64), None)).unwrap();
    assert_eq!(nix.authenticity, "checksum-only");
    let error = sbm_catalog::query::ensure_download_allowed(&nix, false).unwrap_err();
    assert!(
        error.to_string().contains("checksum is not upstream"),
        "{error}"
    );
    sbm_catalog::query::ensure_download_allowed(&nix, true).unwrap();
    let arch = resolve(
        &catalog,
        "arch-linux",
        &selection(Some(CpuArch::X86_64), None),
    )
    .unwrap();
    assert_eq!(arch.authenticity, "unverified-signature");
    assert!(sbm_catalog::query::ensure_download_allowed(&arch, false).is_err());
    let ubuntu = resolve(&catalog, "ubuntu", &selection(Some(CpuArch::X86_64), None)).unwrap();
    assert_eq!(ubuntu.authenticity, "signed-metadata");
    assert_eq!(ubuntu.pinned_key, "ubuntu-cdimage.asc");
    sbm_catalog::query::ensure_download_allowed(&ubuntu, false).unwrap();
    let windows = resolve(&catalog, "windows", &selection(Some(CpuArch::X86_64), None)).unwrap();
    assert_eq!(windows.authenticity, "user-supplied");
}

#[test]
fn declarative_recipes_do_not_execute() {
    let catalog = Catalog::load(&manifest_catalog()).unwrap();
    for (id, mechanism) in [
        ("ubuntu", "autoinstall"),
        ("debian", "preseed"),
        ("fedora", "kickstart"),
    ] {
        let recipe = sbm_catalog::recipe::load_recipe(&manifest_catalog(), id)
            .unwrap()
            .unwrap();
        assert!(!recipe.executed);
        assert_eq!(recipe.mechanism, mechanism);
    }
    let image = resolve(&catalog, "ubuntu", &selection(Some(CpuArch::X86_64), None)).unwrap();
    let plan = installer_plan(&image, &image.mirrors[0]).unwrap();
    assert_plan_does_not_write(&plan).unwrap();
}

#[test]
fn signed_metadata_fixtures_cover_every_authenticated_artifact() {
    if gpg_program().is_err() {
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog = Catalog::load(&manifest_catalog()).unwrap();
    sbm_catalog::upstream::verify_fixture_manifest(
        &catalog,
        &root.join("keys/upstream"),
        &root.join("tests/fixtures"),
    )
    .unwrap();
    let key = fs::read_to_string(root.join("keys/upstream/ubuntu-cdimage.asc")).unwrap();
    let sums = fs::read(root.join("tests/fixtures/ubuntu-26.04-amd64-SHA256SUMS")).unwrap();
    let signature =
        fs::read(root.join("tests/fixtures/ubuntu-26.04-amd64-SHA256SUMS.gpg")).unwrap();
    verify_detached(&key, &signature, &sums).unwrap();
    let mut tampered = sums.clone();
    tampered[10] ^= 0x01;
    assert!(verify_detached(&key, &signature, &tampered).is_err());
}

#[tokio::test]
async fn offline_cache_does_not_use_the_network() {
    let body = b"offline-cache-image".to_vec();
    let tmp = tempfile::tempdir().unwrap();
    let filename = "cached.iso";
    let path = tmp.path().join(filename);
    fs::write(&path, &body).unwrap();
    let sha = sha256_file(&path).unwrap();
    let url = "http://127.0.0.1:1/cached.iso".to_string();
    let retry = url.clone();
    let hit = download(DownloadRequest {
        urls: std::slice::from_ref(&url),
        filename,
        size_bytes: body.len() as u64,
        sha256: &sha,
        cache: tmp.path(),
        allow_loopback_http: true,
        offline: true,
    })
    .await
    .unwrap();
    assert_eq!(fs::read(hit).unwrap(), body);
    fs::remove_file(tmp.path().join(filename)).unwrap();
    let error = download(DownloadRequest {
        urls: std::slice::from_ref(&retry),
        filename,
        size_bytes: body.len() as u64,
        sha256: &sha,
        cache: tmp.path(),
        allow_loopback_http: true,
        offline: true,
    })
    .await
    .unwrap_err();
    assert!(error.to_string().contains("offline"), "{error}");
}

#[tokio::test]
async fn mismatched_partial_journal_restarts_the_download() {
    let body = b"replacement-image-bytes".to_vec();
    let tmp = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let served = body.clone();
    thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut buf = [0_u8; 2048];
        let read = socket.read(&mut buf).unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..read]);
        assert!(
            !request.to_ascii_lowercase().contains("range:"),
            "a mismatched journal must not resume the old partial"
        );
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            served.len()
        );
        socket.write_all(header.as_bytes()).unwrap();
        socket.write_all(&served).unwrap();
    });
    let filename = "sample.iso";
    fs::write(tmp.path().join(filename), b"old-partial").unwrap();
    fs::write(
        tmp.path().join(format!("{filename}.partial.json")),
        "{\"filename\":\"sample.iso\",\"size_bytes\":24,\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}",
    )
    .unwrap();
    let sha = sha256_file(&{
        let full = tmp.path().join("full.iso");
        fs::write(&full, &body).unwrap();
        full
    })
    .unwrap();
    let url = format!("http://127.0.0.1:{port}/{filename}");
    let path = download(DownloadRequest {
        urls: &[url],
        filename,
        size_bytes: body.len() as u64,
        sha256: &sha,
        cache: tmp.path(),
        allow_loopback_http: true,
        offline: false,
    })
    .await
    .unwrap();
    assert_eq!(fs::read(path).unwrap(), body);
}

#[tokio::test]
async fn tuf_rejects_mix_and_match_target_tamper_and_freeze() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog = manifest_catalog();
    let keys = tmp.path().join("keys");
    let out = tmp.path().join("repo");
    let first = publish(PublishRequest::development(
        catalog.clone(),
        keys.clone(),
        out.clone(),
        NonZeroU64::new(1).unwrap(),
    ))
    .await
    .unwrap();
    let snapshot_v1 = fs::read(first.metadata.join("snapshot.json")).unwrap();
    let repo = sbm_catalog::tuf::load_repository(
        &first.metadata,
        &first.targets,
        &tmp.path().join("store-role"),
        true,
    )
    .await
    .unwrap();
    let role = repo
        .delegated_role("community")
        .expect("community delegation");
    assert!(role.terminating);
    assert_eq!(role.name, "community");
    let second = publish(PublishRequest::development(
        catalog.clone(),
        keys,
        out,
        NonZeroU64::new(2).unwrap(),
    ))
    .await
    .unwrap();
    verify_repository(
        &second.metadata,
        &second.targets,
        &tmp.path().join("store-v2"),
        true,
    )
    .await
    .unwrap();
    fs::write(second.metadata.join("snapshot.json"), snapshot_v1).unwrap();
    assert!(
        verify_repository(
            &second.metadata,
            &second.targets,
            &tmp.path().join("store-mix"),
            true,
        )
        .await
        .is_err()
    );

    let tamper_out = tmp.path().join("tamper-repo");
    let tampered = publish(PublishRequest::development(
        catalog.clone(),
        tmp.path().join("tamper-keys"),
        tamper_out,
        NonZeroU64::new(1).unwrap(),
    ))
    .await
    .unwrap();
    let target = fs::read_dir(&tampered.targets)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.is_file())
        .unwrap();
    let mut bytes = fs::read(&target).unwrap();
    bytes[0] ^= 0x1;
    fs::write(&target, bytes).unwrap();
    assert!(
        verify_repository(
            &tampered.metadata,
            &tampered.targets,
            &tmp.path().join("store-target"),
            true,
        )
        .await
        .is_err()
    );

    let freeze_out = tmp.path().join("freeze");
    let mut freeze = PublishRequest::development(
        catalog,
        tmp.path().join("freeze-keys"),
        freeze_out,
        NonZeroU64::new(1).unwrap(),
    );
    freeze.timestamp_lifetime = SignedDuration::from_secs(-60);
    let frozen = publish(freeze).await.unwrap();
    assert!(
        verify_repository(
            &frozen.metadata,
            &frozen.targets,
            &tmp.path().join("store-freeze"),
            true,
        )
        .await
        .is_err()
    );
}
