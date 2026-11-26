use crate::error::{Error, Result, io_at};
use std::path::Path;
use std::process::Command;

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
