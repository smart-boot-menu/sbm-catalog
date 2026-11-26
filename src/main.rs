use clap::{Parser, Subcommand};
use sbm_catalog::catalog::{Catalog, parse_arch, parse_platform};
use sbm_catalog::download::{DownloadRequest, download, import_user_image};
use sbm_catalog::error::{Error, Result};
use sbm_catalog::plan::{assert_plan_does_not_write, installer_plan};
use sbm_catalog::query::{Selection, list_lines, resolve, search};
use sbm_catalog::tuf::{PublishRequest, publish, verify_repository};
use sbm_catalog::upstream::{signed_text_contains_sha256, verify_cleartext, verify_detached};
use std::num::NonZeroU64;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "sbm-catalog",
    version,
    about = "Validate, resolve, download, and publish the Smart Boot Menu OS catalog"
)]
struct Cli {
    #[arg(long, global = true, default_value = "catalog")]
    catalog: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check catalog entries, compatibility profiles, and repository hygiene.
    Validate,
    /// List resolved images.
    List {
        #[arg(long)]
        arch: Option<String>,
        #[arg(long)]
        platform: Option<String>,
    },
    /// Search names, vendors, and identifiers.
    Search {
        query: String,
        #[arg(long)]
        arch: Option<String>,
        #[arg(long)]
        platform: Option<String>,
        #[arg(long)]
        version: Option<String>,
    },
    /// Resolve one image. Fails when the query is ambiguous.
    Resolve {
        query: String,
        #[arg(long)]
        arch: Option<String>,
        #[arg(long)]
        platform: Option<String>,
        #[arg(long)]
        version: Option<String>,
    },
    /// Download one resolved image and verify its size and SHA-256.
    Download {
        query: String,
        #[arg(long, default_value = "build/cache")]
        cache: PathBuf,
        #[arg(long)]
        arch: Option<String>,
        #[arg(long)]
        platform: Option<String>,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        allow_loopback_http: bool,
    },
    /// Load TUF metadata and re-read every target.
    Verify {
        #[arg(long)]
        metadata: PathBuf,
        #[arg(long)]
        targets: PathBuf,
        #[arg(long)]
        datastore: PathBuf,
        #[arg(long)]
        allow_expired: bool,
    },
    /// Sign a development TUF repository. Private keys stay in `--keys`.
    Publish {
        #[arg(long)]
        keys: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 1)]
        version: u64,
    },
    /// Write an install plan that does not name or modify a disk.
    Plan {
        query: String,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        arch: Option<String>,
        #[arg(long)]
        platform: Option<String>,
        #[arg(long)]
        version: Option<String>,
    },
    /// Record the hash of a user-supplied image. The file is not copied into the catalog.
    Import {
        id: String,
        path: PathBuf,
        #[arg(long, default_value = "build/cache")]
        cache: PathBuf,
    },
    /// Copy catalog JSON for offline use. Images are not included.
    ExportOffline {
        destination: PathBuf,
        #[arg(long)]
        metadata: Option<PathBuf>,
    },
    /// Check pinned upstream signatures for Fedora and Debian fixtures.
    UpstreamCheck {
        #[arg(long, default_value = "keys/upstream")]
        keys: PathBuf,
        #[arg(long, default_value = "tests/fixtures")]
        fixtures: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate => {
            let issues = Catalog::load(&cli.catalog)?.validate();
            if issues.is_empty() {
                println!("catalog ok");
                return Ok(());
            }
            for issue in &issues {
                eprintln!("{}: {}", issue.path, issue.message);
            }
            Err(Error::msg(format!("{} catalog issue(s)", issues.len())))
        }
        Command::List { arch, platform } => {
            let catalog = Catalog::load(&cli.catalog)?;
            let selection = selection(arch, platform, None)?;
            println!("{}", list_lines(&search(&catalog, "", &selection)));
            Ok(())
        }
        Command::Search {
            query,
            arch,
            platform,
            version,
        } => {
            let catalog = Catalog::load(&cli.catalog)?;
            let found = search(&catalog, &query, &selection(arch, platform, version)?);
            println!("{}", serde_json::to_string_pretty(&found)?);
            Ok(())
        }
        Command::Resolve {
            query,
            arch,
            platform,
            version,
        } => {
            let catalog = Catalog::load(&cli.catalog)?;
            let image = resolve(&catalog, &query, &selection(arch, platform, version)?)?;
            println!("{}", serde_json::to_string_pretty(&image)?);
            Ok(())
        }
        Command::Download {
            query,
            cache,
            arch,
            platform,
            version,
            allow_loopback_http,
        } => {
            let catalog = Catalog::load(&cli.catalog)?;
            let image = resolve(&catalog, &query, &selection(arch, platform, version)?)?;
            if image.user_supplied_only {
                return Err(Error::msg(format!(
                    "{} is user-supplied; import a local image instead of downloading it",
                    image.entry_id
                )));
            }
            let path = download(DownloadRequest {
                urls: &image.mirrors,
                filename: &image.filename,
                size_bytes: image.size_bytes,
                sha256: &image.sha256,
                cache: &cache,
                allow_loopback_http,
            })
            .await?;
            println!("{}", path.display());
            Ok(())
        }
        Command::Verify {
            metadata,
            targets,
            datastore,
            allow_expired,
        } => {
            let count = verify_repository(&metadata, &targets, &datastore, !allow_expired).await?;
            println!("verified {count} target(s)");
            Ok(())
        }
        Command::Publish { keys, out, version } => {
            let version = NonZeroU64::new(version)
                .ok_or_else(|| Error::msg("version must be greater than zero"))?;
            let published =
                publish(PublishRequest::development(cli.catalog, keys, out, version)).await?;
            println!(
                "published {} target(s) to {}",
                published.target_count,
                published.metadata.display()
            );
            Ok(())
        }
        Command::Plan {
            query,
            source,
            arch,
            platform,
            version,
        } => {
            let catalog = Catalog::load(&cli.catalog)?;
            let image = resolve(&catalog, &query, &selection(arch, platform, version)?)?;
            let source =
                source.unwrap_or_else(|| image.mirrors.first().cloned().unwrap_or_default());
            let plan = installer_plan(&image, &source)?;
            assert_plan_does_not_write(&plan)?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
            Ok(())
        }
        Command::Import { id, path, cache } => {
            let record = import_user_image(&id, &path, &cache)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
            Ok(())
        }
        Command::ExportOffline {
            destination,
            metadata,
        } => {
            sbm_catalog::catalog::export_offline(&cli.catalog, metadata.as_deref(), &destination)?;
            println!("{}", destination.display());
            Ok(())
        }
        Command::UpstreamCheck { keys, fixtures } => upstream_check(&cli.catalog, &keys, &fixtures),
    }
}

fn selection(
    arch: Option<String>,
    platform: Option<String>,
    version: Option<String>,
) -> Result<Selection> {
    let arch = match arch {
        Some(value) => Some(
            parse_arch(&value)
                .ok_or_else(|| Error::msg(format!("unsupported architecture {value}")))?,
        ),
        None => None,
    };
    let platform = match platform {
        Some(value) => Some(
            parse_platform(&value)
                .ok_or_else(|| Error::msg(format!("unsupported platform {value}")))?,
        ),
        None => None,
    };
    Ok(Selection {
        arch,
        platform,
        version,
    })
}

fn upstream_check(
    catalog_root: &std::path::Path,
    keys: &std::path::Path,
    fixtures: &std::path::Path,
) -> Result<()> {
    let fedora_key = std::fs::read_to_string(keys.join("fedora-45.asc"))
        .map_err(|err| sbm_catalog::error::io_at(keys.join("fedora-45.asc"), err))?;
    let fedora =
        std::fs::read(fixtures.join("fedora-45-server-x86_64.CHECKSUM")).map_err(|err| {
            sbm_catalog::error::io_at(fixtures.join("fedora-45-server-x86_64.CHECKSUM"), err)
        })?;
    verify_cleartext(&fedora_key, &fedora)?;
    let fedora_text = String::from_utf8_lossy(&fedora);
    let debian_key = std::fs::read_to_string(keys.join("debian-cd.asc"))
        .map_err(|err| sbm_catalog::error::io_at(keys.join("debian-cd.asc"), err))?;
    let debian_sums =
        std::fs::read(fixtures.join("debian-13.8.0-amd64-SHA256SUMS")).map_err(|err| {
            sbm_catalog::error::io_at(fixtures.join("debian-13.8.0-amd64-SHA256SUMS"), err)
        })?;
    let debian_sig =
        std::fs::read(fixtures.join("debian-13.8.0-amd64-SHA256SUMS.sign")).map_err(|err| {
            sbm_catalog::error::io_at(fixtures.join("debian-13.8.0-amd64-SHA256SUMS.sign"), err)
        })?;
    verify_detached(&debian_key, &debian_sig, &debian_sums)?;
    let catalog = Catalog::load(catalog_root)?;
    let fedora_entry = catalog
        .get("fedora")
        .ok_or_else(|| Error::msg("fedora is not in the catalog"))?;
    for artifact in fedora_entry
        .entry
        .releases
        .iter()
        .flat_map(|release| &release.artifacts)
    {
        if artifact.filename.contains("x86_64")
            && !signed_text_contains_sha256(&fedora_text, &artifact.filename, &artifact.sha256)
        {
            return Err(Error::msg(format!(
                "{} is not in the signed Fedora CHECKSUM",
                artifact.filename
            )));
        }
    }
    let debian_text = String::from_utf8_lossy(&debian_sums);
    let debian_entry = catalog
        .get("debian")
        .ok_or_else(|| Error::msg("debian is not in the catalog"))?;
    for artifact in debian_entry
        .entry
        .releases
        .iter()
        .flat_map(|release| &release.artifacts)
    {
        if artifact.filename.contains("amd64")
            && !signed_text_contains_sha256(&debian_text, &artifact.filename, &artifact.sha256)
        {
            return Err(Error::msg(format!(
                "{} is not in the signed Debian SHA256SUMS",
                artifact.filename
            )));
        }
    }
    println!("upstream signatures match the pinned Fedora and Debian catalog hashes");
    Ok(())
}
