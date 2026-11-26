use crate::catalog::artifact_url;
use crate::error::{Error, Result, io_at};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DownloadRequest<'a> {
    pub urls: &'a [String],
    pub filename: &'a str,
    pub size_bytes: u64,
    pub sha256: &'a str,
    pub cache: &'a Path,
    pub allow_loopback_http: bool,
    /// Return a complete verified cache file and do not open the network.
    pub offline: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PartialJournal {
    filename: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportRecord {
    pub catalog_entry_id: String,
    pub filename: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub source_path: String,
}

fn journal_matches(path: &Path, request: &DownloadRequest<'_>) -> Result<bool> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(io_at(path, err)),
    };
    let journal: PartialJournal = serde_json::from_slice(&bytes).unwrap_or(PartialJournal {
        filename: String::new(),
        size_bytes: 0,
        sha256: String::new(),
    });
    Ok(journal.filename == request.filename
        && journal.size_bytes == request.size_bytes
        && journal.sha256.eq_ignore_ascii_case(request.sha256))
}

fn write_journal(path: &Path, request: &DownloadRequest<'_>) -> Result<()> {
    let journal = PartialJournal {
        filename: request.filename.to_string(),
        size_bytes: request.size_bytes,
        sha256: request.sha256.to_string(),
    };
    let bytes = serde_json::to_vec_pretty(&journal).map_err(|err| Error::msg(err.to_string()))?;
    fs::write(path, bytes).map_err(|err| io_at(path, err))
}

pub async fn download(request: DownloadRequest<'_>) -> Result<PathBuf> {
    if request.size_bytes == 0 || request.sha256.chars().all(|ch| ch == '0') {
        return Err(Error::msg(
            "refusing to download an artifact without a pinned size and sha256",
        ));
    }
    fs::create_dir_all(request.cache).map_err(|err| io_at(request.cache, err))?;
    let destination = request.cache.join(request.filename);
    let journal_file = request
        .cache
        .join(format!("{}.partial.json", request.filename));
    let mut offset = existing_len(&destination)?;
    if offset > request.size_bytes {
        fs::remove_file(&destination).map_err(|err| io_at(&destination, err))?;
        offset = 0;
    }
    if offset == request.size_bytes {
        if verify_file(&destination, request.size_bytes, request.sha256).is_ok() {
            fs::remove_file(&journal_file).ok();
            return Ok(destination);
        }
        fs::remove_file(&destination).map_err(|io| io_at(&destination, io))?;
        offset = 0;
        if request.offline {
            return Err(Error::msg(format!(
                "offline cache failed verification for {}",
                request.filename
            )));
        }
    }
    if offset > 0 && !journal_matches(&journal_file, &request)? {
        fs::remove_file(&destination).ok();
        offset = 0;
    }
    if request.offline {
        return Err(Error::msg(format!(
            "offline cache does not contain a complete verified {}",
            request.filename
        )));
    }
    write_journal(&journal_file, &request)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .connect_timeout(Duration::from_secs(20))
        .build()
        .map_err(|err| Error::msg(err.to_string()))?;
    let mut last_error = Error::msg("no mirrors were attempted");
    for mirror in request.urls {
        let url = artifact_url(mirror, request.filename);
        if let Err(err) = ensure_url_allowed(&url, request.allow_loopback_http) {
            last_error = err;
            continue;
        }
        match download_one(&client, &url, &destination, request.size_bytes, offset).await {
            Ok(written) => {
                offset = written;
                if offset == request.size_bytes {
                    if let Err(err) = verify_file(&destination, request.size_bytes, request.sha256)
                    {
                        fs::remove_file(&destination).map_err(|io| io_at(&destination, io))?;
                        return Err(err);
                    }
                    fs::remove_file(&journal_file).ok();
                    return Ok(destination);
                }
                last_error = Error::msg(format!(
                    "incomplete download of {} ({offset} of {} bytes)",
                    request.filename, request.size_bytes
                ));
            }
            Err(err) => last_error = err,
        }
    }
    Err(last_error)
}

async fn download_one(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    expected: u64,
    mut offset: u64,
) -> Result<u64> {
    for _ in 0..8 {
        if offset == expected {
            return Ok(offset);
        }
        let mut builder = client.get(url).timeout(Duration::from_secs(120));
        if offset > 0 {
            builder = builder.header(reqwest::header::RANGE, format!("bytes={offset}-"));
        }
        let response = builder
            .send()
            .await
            .map_err(|err| Error::msg(err.to_string()))?;
        if offset > 0 && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            fs::remove_file(destination).ok();
            offset = 0;
            continue;
        }
        if offset == 0 && !response.status().is_success() {
            return Err(Error::msg(format!("{url} returned {}", response.status())));
        }
        if let Some(length) = response.content_length() {
            let remaining = expected - offset;
            if length != remaining {
                return Err(Error::msg(format!(
                    "{url} content length {length} does not match the remaining {remaining} bytes"
                )));
            }
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(offset > 0)
            .write(true)
            .open(destination)
            .map_err(|err| io_at(destination, err))?;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|err| Error::msg(err.to_string()))?;
            file.write_all(&chunk)
                .map_err(|err| io_at(destination, err))?;
            offset += chunk.len() as u64;
            if offset > expected {
                let _ = fs::remove_file(destination);
                return Err(Error::msg(format!("{url} exceeded the catalog size")));
            }
        }
        if offset == expected {
            return Ok(offset);
        }
    }
    Ok(offset)
}

pub fn verify_file(path: &Path, size_bytes: u64, sha256: &str) -> Result<()> {
    let actual = existing_len(path)?;
    if actual != size_bytes {
        return Err(Error::msg(format!(
            "{} is {actual} bytes, catalog expects {size_bytes}",
            path.display()
        )));
    }
    let digest = sha256_file(path)?;
    if !digest.eq_ignore_ascii_case(sha256) {
        return Err(Error::msg(format!(
            "{} sha256 {digest} does not match catalog {sha256}",
            path.display()
        )));
    }
    Ok(())
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(|err| io_at(path, err))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|err| io_at(path, err))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn import_user_image(entry_id: &str, source: &Path, cache: &Path) -> Result<ImportRecord> {
    let filename = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::msg("import path has no file name"))?
        .to_string();
    let size_bytes = existing_len(source)?;
    let record = ImportRecord {
        catalog_entry_id: entry_id.to_string(),
        filename,
        size_bytes,
        sha256: sha256_file(source)?,
        source_path: source.display().to_string(),
    };
    let dir = cache.join("imports");
    fs::create_dir_all(&dir).map_err(|err| io_at(&dir, err))?;
    let path = dir.join(format!("{entry_id}.json"));
    fs::write(&path, serde_json::to_vec_pretty(&record)?).map_err(|err| io_at(&path, err))?;
    Ok(record)
}

pub fn read_import(cache: &Path, entry_id: &str) -> Result<Option<ImportRecord>> {
    let path = cache.join("imports").join(format!("{entry_id}.json"));
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|err| io_at(&path, err))?;
    Ok(Some(serde_json::from_str(&text)?))
}

fn existing_len(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    Ok(fs::metadata(path).map_err(|err| io_at(path, err))?.len())
}

pub fn ensure_url_allowed(url: &str, allow_loopback_http: bool) -> Result<()> {
    let parsed = url::Url::parse(url).map_err(|err| Error::msg(err.to_string()))?;
    match parsed.scheme() {
        "https" => Ok(()),
        "http"
            if allow_loopback_http
                && matches!(parsed.host_str(), Some("127.0.0.1" | "localhost")) =>
        {
            Ok(())
        }
        _ => Err(Error::msg(format!("refusing non-https URL {url}"))),
    }
}
