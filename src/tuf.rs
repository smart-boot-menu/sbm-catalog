use crate::catalog::Catalog;
use crate::error::{Error, Result, io_at};
use aws_lc_rs::rand::SystemRandom;
use aws_lc_rs::signature::Ed25519KeyPair;
use jiff::{SignedDuration, Timestamp};
use std::collections::HashMap;
use std::fs;
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use tough::editor::RepositoryEditor;
use tough::editor::signed::{PathExists, SignedRole};
use tough::key_source::LocalKeySource;
use tough::schema::{KeyHolder, PathPattern, PathSet, RoleKeys, RoleType, Root};
use tough::sign::{Sign, parse_keypair};
use tough::{ExpirationEnforcement, FilesystemTransport, IntoVec, Repository, RepositoryLoader};
use url::Url;

const SPEC_VERSION: &str = "1.0.0";
const ROLE_FILES: &[(&str, RoleType)] = &[
    ("root", RoleType::Root),
    ("targets", RoleType::Targets),
    ("snapshot", RoleType::Snapshot),
    ("timestamp", RoleType::Timestamp),
];

#[derive(Debug, Clone)]
pub struct PublishRequest {
    pub catalog: PathBuf,
    pub key_dir: PathBuf,
    pub out_dir: PathBuf,
    pub version: NonZeroU64,
    pub targets_lifetime: SignedDuration,
    pub snapshot_lifetime: SignedDuration,
    pub timestamp_lifetime: SignedDuration,
    pub delegate_community: bool,
}

impl PublishRequest {
    pub fn development(
        catalog: PathBuf,
        key_dir: PathBuf,
        out_dir: PathBuf,
        version: NonZeroU64,
    ) -> Self {
        Self {
            catalog,
            key_dir,
            out_dir,
            version,
            targets_lifetime: SignedDuration::from_secs(30 * 24 * 60 * 60),
            snapshot_lifetime: SignedDuration::from_secs(7 * 24 * 60 * 60),
            timestamp_lifetime: SignedDuration::from_secs(24 * 60 * 60),
            delegate_community: true,
        }
    }
}

#[derive(Debug)]
pub struct PublishedRepository {
    pub metadata: PathBuf,
    pub targets: PathBuf,
    pub trusted_root: PathBuf,
    pub target_count: usize,
}

pub async fn publish(request: PublishRequest) -> Result<PublishedRepository> {
    if request.key_dir.starts_with(&request.catalog)
        || request.out_dir.starts_with(&request.catalog)
    {
        return Err(Error::msg(
            "signing keys and repository output must stay outside the catalog tree",
        ));
    }
    let catalog = Catalog::load(&request.catalog)?;
    let issues = catalog.validate();
    if !issues.is_empty() {
        return Err(Error::msg(format!(
            "refusing to publish an invalid catalog: {}",
            issues[0].message
        )));
    }
    fs::create_dir_all(&request.key_dir).map_err(|err| io_at(&request.key_dir, err))?;
    ensure_keys(&request.key_dir).await?;
    let staging = request.out_dir.join("staging");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|err| io_at(&staging, err))?;
    }
    fs::create_dir_all(&staging).map_err(|err| io_at(&staging, err))?;
    let mut staged = Vec::new();
    for loaded in &catalog.entries {
        staged.push(stage(&loaded.path, &staging, "entry")?);
        staged.push(stage(&loaded.profile_path, &staging, "compat")?);
    }
    let recipes = request.catalog.join("recipes");
    if recipes.is_dir() {
        let mut paths: Vec<_> = fs::read_dir(&recipes)
            .map_err(|err| io_at(&recipes, err))?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect();
        paths.sort();
        for path in paths {
            staged.push(stage(&path, &staging, "recipe")?);
        }
    }
    let root_path = request.key_dir.join("1.root.json");
    let mut editor = RepositoryEditor::new(&root_path).await.map_err(tough_err)?;
    for path in &staged {
        editor.add_target_path(path).await.map_err(tough_err)?;
    }
    let expires = |lifetime: SignedDuration| -> Result<Timestamp> {
        Timestamp::now()
            .checked_add(lifetime)
            .map_err(|err| Error::msg(err.to_string()))
    };
    editor
        .targets_version(request.version)
        .map_err(tough_err)?
        .targets_expires(expires(request.targets_lifetime)?)
        .map_err(tough_err)?;
    if request.delegate_community {
        let community = Box::new(LocalKeySource {
            path: request.key_dir.join("community.pkcs8"),
        });
        editor
            .delegate_role(
                "community",
                &[community],
                PathSet::Paths(vec![PathPattern::new("community/*").map_err(tough_err)?]),
                true,
                NonZeroU64::new(1).expect("threshold"),
                expires(request.targets_lifetime)?,
                request.version,
            )
            .await
            .map_err(tough_err)?;
    }
    editor.snapshot_version(request.version);
    editor.snapshot_expires(expires(request.snapshot_lifetime)?);
    editor.timestamp_version(request.version);
    editor.timestamp_expires(expires(request.timestamp_lifetime)?);
    let keys = role_sources(&request.key_dir);
    let signed = editor.sign(&keys).await.map_err(tough_err)?;
    let metadata = request.out_dir.join("metadata");
    let targets = request.out_dir.join("targets");
    fs::create_dir_all(&metadata).map_err(|err| io_at(&metadata, err))?;
    fs::create_dir_all(&targets).map_err(|err| io_at(&targets, err))?;
    signed.write(&metadata).await.map_err(tough_err)?;
    signed
        .copy_targets(&staging, &targets, PathExists::Replace)
        .await
        .map_err(tough_err)?;
    let trusted_root = metadata.join(format!("{}.root.json", 1));
    if !trusted_root.is_file() {
        fs::copy(&root_path, &trusted_root).map_err(|err| io_at(&root_path, err))?;
    }
    Ok(PublishedRepository {
        metadata,
        targets,
        trusted_root,
        target_count: staged.len(),
    })
}

pub async fn verify_repository(
    metadata: &Path,
    targets: &Path,
    datastore: &Path,
    enforce_expiration: bool,
) -> Result<usize> {
    let repo = load_repository(metadata, targets, datastore, enforce_expiration).await?;
    let names: Vec<_> = repo.all_targets().map(|(name, _)| name.clone()).collect();
    if names.is_empty() {
        return Err(Error::msg("TUF targets metadata lists no catalog files"));
    }
    for name in &names {
        let stream = repo
            .read_target(name)
            .await
            .map_err(tough_err)?
            .ok_or_else(|| Error::msg(format!("missing TUF target {name:?}")))?;
        let bytes = stream.into_vec().await.map_err(tough_err)?;
        if bytes.is_empty() {
            return Err(Error::msg(format!("TUF target {name:?} was empty")));
        }
    }
    Ok(names.len())
}

pub async fn load_repository(
    metadata: &Path,
    targets: &Path,
    datastore: &Path,
    enforce_expiration: bool,
) -> Result<Repository> {
    let root_path = metadata.join("1.root.json");
    let root = fs::read(&root_path).map_err(|err| io_at(&root_path, err))?;
    fs::create_dir_all(datastore).map_err(|err| io_at(datastore, err))?;
    let enforcement = if enforce_expiration {
        ExpirationEnforcement::Safe
    } else {
        ExpirationEnforcement::Unsafe
    };
    RepositoryLoader::new(&root, dir_url(metadata)?, dir_url(targets)?)
        .transport(FilesystemTransport)
        .datastore(datastore)
        .expiration_enforcement(enforcement)
        .load()
        .await
        .map_err(tough_err)
}

async fn ensure_keys(key_dir: &Path) -> Result<()> {
    let root_path = key_dir.join("1.root.json");
    if root_path.is_file() {
        for (name, _) in ROLE_FILES {
            let path = key_dir.join(format!("{name}.pkcs8"));
            if !path.is_file() {
                return Err(Error::msg(format!(
                    "root metadata exists but {} is missing",
                    path.display()
                )));
            }
        }
        let community = key_dir.join("community.pkcs8");
        if !community.is_file() {
            write_key(&community)?;
        }
        return Ok(());
    }
    let mut keys = HashMap::new();
    let mut roles = HashMap::new();
    for (name, role) in ROLE_FILES {
        let path = key_dir.join(format!("{name}.pkcs8"));
        let der = write_key(&path)?;
        let parsed = parse_keypair(&der).map_err(tough_err)?;
        let key = parsed.tuf_key();
        let key_id = key.key_id().map_err(|err| Error::msg(err.to_string()))?;
        roles.insert(
            *role,
            RoleKeys {
                keyids: vec![key_id.clone()],
                threshold: NonZeroU64::new(1).expect("threshold"),
                _extra: HashMap::new(),
            },
        );
        keys.insert(key_id, key);
    }
    write_key(&key_dir.join("community.pkcs8"))?;
    let root = Root {
        spec_version: SPEC_VERSION.to_string(),
        consistent_snapshot: false,
        version: NonZeroU64::new(1).expect("root version"),
        expires: Timestamp::now()
            .checked_add(SignedDuration::from_secs(365 * 24 * 60 * 60))
            .map_err(|err| Error::msg(err.to_string()))?,
        keys,
        roles,
        _extra: HashMap::new(),
    };
    let source: Vec<Box<dyn tough::key_source::KeySource>> = vec![Box::new(LocalKeySource {
        path: key_dir.join("root.pkcs8"),
    })];
    let signed = SignedRole::new(
        root.clone(),
        &KeyHolder::Root(root),
        &source,
        &SystemRandom::new(),
    )
    .await
    .map_err(tough_err)?;
    fs::write(&root_path, signed.buffer()).map_err(|err| io_at(&root_path, err))?;
    Ok(())
}

fn write_key(path: &Path) -> Result<Vec<u8>> {
    if path.is_file() {
        return fs::read(path).map_err(|err| io_at(path, err));
    }
    let document = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
        .map_err(|_| Error::msg("Ed25519 key generation failed"))?;
    fs::write(path, document.as_ref()).map_err(|err| io_at(path, err))?;
    Ok(document.as_ref().to_vec())
}

fn role_sources(key_dir: &Path) -> Vec<Box<dyn tough::key_source::KeySource>> {
    ROLE_FILES
        .iter()
        .map(|(name, _)| {
            Box::new(LocalKeySource {
                path: key_dir.join(format!("{name}.pkcs8")),
            }) as Box<dyn tough::key_source::KeySource>
        })
        .collect()
}

fn stage(source: &Path, staging: &Path, prefix: &str) -> Result<PathBuf> {
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::msg(format!("{} has no file name", source.display())))?;
    let destination = staging.join(format!("{prefix}-{name}"));
    fs::copy(source, &destination).map_err(|err| io_at(source, err))?;
    Ok(destination)
}

fn dir_url(path: &Path) -> Result<Url> {
    let absolute = path.canonicalize().map_err(|err| io_at(path, err))?;
    Url::from_directory_path(absolute).map_err(|_| Error::msg("path cannot be used as a file URL"))
}

fn tough_err(err: impl ToString) -> Error {
    Error::msg(err.to_string())
}
