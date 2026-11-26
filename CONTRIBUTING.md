# Contributing catalog entries

Add one JSON file per operating system under `catalog/linux/`, `catalog/bsd/`, `catalog/windows/`, or `catalog/recovery/`. The file name is `<id>.json` and the `id` inside the file is the same slug. Add `catalog/compatibility/<id>.json` for the license, image format, boot methods, and unattended mechanisms.

Use `OsCatalogEntry` schema `1.0`. Do not add fields to that object that `sbm-types` does not know. Put extra description in the compatibility profile.

## Required facts

- HTTPS homepage and HTTPS mirrors. No userinfo in URLs. Directory mirrors may end in `/`; the file name is appended.
- SHA-256 of 64 lowercase hex characters and a size greater than zero, unless the artifact is `user_supplied_only`. User-supplied artifacts have an empty mirror list and either a real hash and size or a zero size with an all-zero SHA-256. The all-zero hash means "not pinned". `download` will not fetch it.
- Architectures are `x86_64`, `aarch64`, `ia32`, or `riscv64`. Firmware platforms are `uefi-native`, `uefi-uboot`, `apple-silicon-m1n1-uboot`, or `legacy-bios`.
- Split a release when one architecture boots firmware that another does not. `aarch64` does not imply Apple Silicon. `apple-silicon-m1n1-uboot` requires `apple_silicon` set to `explicit-m1n1-uboot-artifact` and an actual `m1n1` plus U-Boot artifact.
- Record the unattended mechanism and `stable`, `experimental`, or `unsupported`. That record does not run the mechanism.
- State the license and the limitations, including when a signature URL has not been verified.

## Rejected contributions

The validator rejects duplicate ids, duplicate `(version, architecture)` pairs, HTTP mirrors, unknown architectures, missing hashes, install strategies that do not match the OS family, and any file in the catalog tree that is not `.json` or `.gitkeep`. It also rejects shebangs, `curl |`, `wget |`, and private-key blocks. A TUF delegation does not override those checks.

Do not commit private signing keys, ISO files, or Windows media. Checksums copied from the same untrusted place as an image need a separate signed metadata check before they count as authenticity. Say so in `docs/UPSTREAM_PROVENANCE.md` when you pin a release.

## Check

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
sbm-catalog validate
```
