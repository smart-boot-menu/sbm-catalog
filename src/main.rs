use clap::{Parser, Subcommand};
use sbm_catalog::catalog::{Catalog, parse_arch, parse_platform};
use sbm_catalog::download::{DownloadRequest, download, import_user_image};
use sbm_catalog::error::{Error, Result};
use sbm_catalog::plan::{assert_plan_does_not_write, installer_plan};
use sbm_catalog::query::{Selection, ensure_download_allowed, list_lines, resolve, search};
use sbm_catalog::recipe::load_recipe;
use sbm_catalog::tuf::{PublishRequest, publish, verify_repository};
use sbm_catalog::upstream::verify_fixture_manifest;
use serde::Serialize;
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
        /// Download a checksum-only or unverified-signature image.
        ///
        /// A checksum from the same place as the image is not upstream authentication.
        #[arg(long)]
        allow_unauthenticated: bool,
        /// Use a complete verified file in the cache and do not open the network.
        #[arg(long)]
        offline: bool,
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
    /// Re-verify signed upstream checksum documents against the catalog hashes.
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
            allow_unauthenticated,
            offline,
        } => {
            let catalog = Catalog::load(&cli.catalog)?;
            let image = resolve(&catalog, &query, &selection(arch, platform, version)?)?;
            ensure_download_allowed(&image, allow_unauthenticated)?;
            let path = download(DownloadRequest {
                urls: &image.mirrors,
                filename: &image.filename,
                size_bytes: image.size_bytes,
                sha256: &image.sha256,
                cache: &cache,
                allow_loopback_http,
                offline,
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
            let recipe = load_recipe(&cli.catalog, &image.entry_id)?;
            let document = PlanDocument {
                executed: false,
                plan,
                recipe,
            };
            println!("{}", serde_json::to_string_pretty(&document)?);
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
    let catalog = Catalog::load(catalog_root)?;
    verify_fixture_manifest(&catalog, keys, fixtures)?;
    println!(
        "upstream signatures match the catalog hashes; checksum-only and unverified-signature entries stay download-disabled"
    );
    Ok(())
}

#[derive(Serialize)]
struct PlanDocument {
    executed: bool,
    plan: sbm_types::OsInstallPlan,
    recipe: Option<sbm_catalog::recipe::InstallRecipe>,
}
