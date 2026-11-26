use crate::catalog::Catalog;
use crate::error::{Error, Result, io_at};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct FixtureManifest {
    checks: Vec<FixtureCheck>,
}

#[derive(Debug, Deserialize)]
struct FixtureCheck {
    key: String,
    message: String,
    #[serde(default)]
    signature: Option<String>,
    cleartext: bool,
    catalog_id: String,
    filename: String,
    algorithm: String,
}

/// Re-verify every signed checksum document and require the catalog hash to be inside it.
pub fn verify_fixture_manifest(catalog: &Catalog, keys: &Path, fixtures: &Path) -> Result<()> {
    let manifest_path = fixtures.join("manifest.json");
    let manifest_bytes = std::fs::read(&manifest_path).map_err(|err| io_at(&manifest_path, err))?;
    let manifest: FixtureManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| Error::msg(format!("{}: {err}", manifest_path.display())))?;
    if manifest.checks.is_empty() {
        return Err(Error::msg("upstream fixture manifest has no checks"));
    }
    for check in &manifest.checks {
        let key_path = keys.join(&check.key);
        let key = std::fs::read_to_string(&key_path).map_err(|err| io_at(&key_path, err))?;
        let message_path = fixtures.join(&check.message);
        let message = std::fs::read(&message_path).map_err(|err| io_at(&message_path, err))?;
        if check.cleartext {
            verify_cleartext(&key, &message)?;
        } else {
            let signature_name = check.signature.as_deref().ok_or_else(|| {
                Error::msg(format!(
                    "{} needs a detached signature file",
                    check.filename
                ))
            })?;
            let signature_path = fixtures.join(signature_name);
            let signature =
                std::fs::read(&signature_path).map_err(|err| io_at(&signature_path, err))?;
            verify_detached(&key, &signature, &message)?;
        }
        let loaded = catalog
            .get(&check.catalog_id)
            .ok_or_else(|| Error::msg(format!("{} is not in the catalog", check.catalog_id)))?;
        let artifact = loaded
            .entry
            .releases
            .iter()
            .flat_map(|release| &release.artifacts)
            .find(|artifact| artifact.filename == check.filename)
            .ok_or_else(|| Error::msg(format!("{} is not a catalog artifact", check.filename)))?;
        let text = String::from_utf8_lossy(&message);
        let present = match check.algorithm.as_str() {
            "sha256" => signed_text_contains_sha256(&text, &artifact.filename, &artifact.sha256),
            "sha512" => {
                let sha512 = artifact.sha512.as_deref().ok_or_else(|| {
                    Error::msg(format!("{} has no catalog sha512", artifact.filename))
                })?;
                text_contains_sha512(&text, &artifact.filename, sha512)
            }
            other => return Err(Error::msg(format!("unsupported fixture algorithm {other}"))),
        };
        if !present {
            return Err(Error::msg(format!(
                "{} catalog hash is not in the signed document {}",
                check.filename, check.message
            )));
        }
        let profile = loaded.profile.artifact(&artifact.filename).ok_or_else(|| {
            Error::msg(format!(
                "{} has no compatibility profile",
                artifact.filename
            ))
        })?;
        if profile.authenticity != "signed-metadata" || profile.pinned_key != check.key {
            return Err(Error::msg(format!(
                "{} must be signed-metadata pinned to {}",
                artifact.filename, check.key
            )));
        }
    }
    for loaded in &catalog.entries {
        for artifact in &loaded.profile.artifacts {
            if artifact.authenticity == "signed-metadata"
                && !manifest
                    .checks
                    .iter()
                    .any(|check| check.filename == artifact.filename)
            {
                return Err(Error::msg(format!(
                    "{} is signed-metadata but has no upstream fixture",
                    artifact.filename
                )));
            }
        }
    }
    Ok(())
}

/// Confirm a pinned OpenPGP key signs `message`.
///
/// The key is imported into an empty temporary keyring. A checksum published next
/// to an image is not treated as proof unless this signature check succeeds.
pub fn verify_detached(public_key_armor: &str, signature: &[u8], message: &[u8]) -> Result<()> {
    let home = tempfile::tempdir().map_err(|err| Error::msg(err.to_string()))?;
    let key_path = home.path().join("key.asc");
    let sig_path = home.path().join("sig");
    let msg_path = home.path().join("msg");
    std::fs::write(&key_path, public_key_armor).map_err(|err| io_at(&key_path, err))?;
    std::fs::write(&sig_path, signature).map_err(|err| io_at(&sig_path, err))?;
    std::fs::write(&msg_path, message).map_err(|err| io_at(&msg_path, err))?;
    gpg(
        home.path(),
        &["--import", key_path.to_str().unwrap_or("key.asc")],
    )?;
    gpg(
        home.path(),
        &[
            "--verify",
            sig_path.to_str().unwrap_or("sig"),
            msg_path.to_str().unwrap_or("msg"),
        ],
    )
}

pub fn verify_cleartext(public_key_armor: &str, signed_message: &[u8]) -> Result<()> {
    let home = tempfile::tempdir().map_err(|err| Error::msg(err.to_string()))?;
    let key_path = home.path().join("key.asc");
    let msg_path = home.path().join("msg");
    std::fs::write(&key_path, public_key_armor).map_err(|err| io_at(&key_path, err))?;
    std::fs::write(&msg_path, signed_message).map_err(|err| io_at(&msg_path, err))?;
    gpg(
        home.path(),
        &["--import", key_path.to_str().unwrap_or("key.asc")],
    )?;
    gpg(
        home.path(),
        &["--verify", msg_path.to_str().unwrap_or("msg")],
    )
}

fn gpg(home: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new(gpg_program()?)
        .arg("--batch")
        .arg("--yes")
        .arg("--homedir")
        .arg(home)
        .arg("--trust-model")
        .arg("always")
        .args(args)
        .output()
        .map_err(|err| Error::msg(format!("gpg failed to start: {err}")))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(Error::msg(format!("OpenPGP verification failed: {stderr}")))
    }
}

pub fn gpg_program() -> Result<&'static str> {
    for candidate in [
        "/opt/homebrew/bin/gpg",
        "/usr/local/bin/gpg",
        "/usr/bin/gpg",
    ] {
        if Path::new(candidate).is_file() {
            return Ok(candidate);
        }
    }
    Err(Error::msg(
        "gpg was not found; upstream signature checks need GnuPG",
    ))
}

pub fn signed_text_contains_sha256(document: &str, filename: &str, sha256: &str) -> bool {
    let wanted = sha256.to_ascii_lowercase();
    for line in document.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("-----")
            || line.starts_with("Hash:")
        {
            continue;
        }
        let (hash, name) = if let Some(rest) = line.strip_prefix("SHA256 (") {
            let Some((name, hash)) = rest.split_once(") = ") else {
                continue;
            };
            (hash.trim(), name.trim())
        } else {
            let mut parts = line.split_whitespace();
            let Some(hash) = parts.next() else { continue };
            let Some(name) = parts.next() else { continue };
            (hash, name.trim_start_matches('*'))
        };
        if name == filename && hash.eq_ignore_ascii_case(&wanted) {
            return true;
        }
    }
    false
}

pub fn text_contains_sha512(document: &str, filename: &str, sha512: &str) -> bool {
    let wanted = sha512.to_ascii_lowercase();
    for line in document.lines() {
        let mut parts = line.split_whitespace();
        let Some(hash) = parts.next() else { continue };
        let Some(name) = parts.next() else { continue };
        if name.trim_start_matches('*') == filename && hash.eq_ignore_ascii_case(&wanted) {
            return true;
        }
    }
    false
}
