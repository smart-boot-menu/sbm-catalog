# Operating System Catalog (`sbm-catalog`)

Declarative operating system catalog for Smart Boot Menu. Entries describe where an official image lives, how to check it, which machines can boot it, and which unattended mechanisms exist. The tools resolve those entries, download and hash images, and publish the catalog as a [TUF](https://theupdateframework.github.io/) repository.

The catalog does not install an operating system and does not write a physical disk. `sbm-catalog plan` emits an `OsInstallPlan` whose storage target is `unbound` and whose operation list is empty, plus a declarative recipe when one exists. Ubuntu, Debian, and Fedora recipes set `executed` to false. Automated installation is not run.

## Catalog

Entries live in `catalog/{linux,bsd,windows,recovery}/` and use `OsCatalogEntry` schema 1.0 from `sbm-common` (`28d38450954f0b93815e12f31da92370b13a7462`). Licensing, per-image architecture, boot method, and unattended-mechanism notes live in `catalog/compatibility/<id>.json`. That sidecar is additive. It is not a second copy of `OsCatalogEntry`.

Pinned releases, retrieved on 2026-11-26:

| Entry | Version | Architectures |
| --- | --- | --- |
| Ubuntu | 26.04.1 live server | x86_64, aarch64 |
| Debian | 13.8.0 netinst | x86_64, aarch64 |
| Fedora | 45 Server netinst, compose 45-1.4 | x86_64, aarch64 |
| Arch Linux | 2026.11.01 | x86_64 |
| Linux Mint | 22.3 Cinnamon | x86_64 |
| NixOS | 26.05.15079.0dc498bfe49b minimal | x86_64, aarch64 |
| openSUSE Leap | 16.1 online installer | x86_64, aarch64 |
| Windows | 11, user-supplied ISO | x86_64 |
| FreeBSD | 15.1-RELEASE bootonly | x86_64, aarch64 |
| Proxmox VE | 9.3-1 | x86_64 |
| SystemRescue | 13.03 | x86_64 |
| GParted Live | 1.8.1-6 | x86_64 |

`aarch64` means generic UEFI. None of these images are offered for `apple-silicon-m1n1-uboot`. `list --arch` and `list --platform` drop images that do not match.

Each compatibility artifact has an `authenticity` value. `signed-metadata` is a pinned key over a checksum document. `unverified-signature` is a signature URL whose payload was not checked against the image. `checksum-only` is a hash with no signature. `user-supplied` is Windows. `download` fetches only `signed-metadata` and `signed-image` unless `--allow-unauthenticated` is set. That flag does not fetch Windows. `--offline` uses a complete verified file in the cache and does not open the network. A partial download records `cache/<filename>.partial.json`; a journal that names a different hash deletes the partial and starts over.

Windows images are not downloaded; `import` records a local hash. See [docs/UPSTREAM_PROVENANCE.md](docs/UPSTREAM_PROVENANCE.md) and [docs/LIMITATIONS.md](docs/LIMITATIONS.md).

## Commands

```sh
sbm-catalog validate
sbm-catalog list --arch aarch64 --platform uefi-native
sbm-catalog search ubuntu
sbm-catalog resolve ubuntu --arch x86_64
sbm-catalog download debian --arch aarch64 --platform uefi-native
sbm-catalog download nixos --arch x86_64
sbm-catalog download debian --arch aarch64 --offline
sbm-catalog plan ubuntu --arch x86_64
sbm-catalog import windows /path/to/image.iso
sbm-catalog export-offline build/offline
sbm-catalog publish --keys build/dev-keys --out build/repository
sbm-catalog verify --metadata build/repository/metadata --targets build/repository/targets --datastore build/tuf-store
sbm-catalog upstream-check
```

`tools/catalog-check` runs validation through Cargo. `tools/boot-installer.sh` boots a hash-checked ISO in a disposable QEMU virtual machine and does not take a host disk.

Development TUF keys are created under the directory you pass to `--keys`. They are PKCS#8 files, gitignored, and must not be committed. Production signing is described in [docs/TUF.md](docs/TUF.md).

## Tests

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## License

Original code and documentation in this repository are dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE). Operating system images keep the licenses recorded in their compatibility profiles. This repository does not contain those images.
