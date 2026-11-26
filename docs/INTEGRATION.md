# Integration

## Agent 02 — `sbm-core`

Consume `OsCatalogEntry` and `OsInstallPlan` from `sbm-types` at `sbm-common` revision `28d38450954f0b93815e12f31da92370b13a7462`. Do not parse a private catalog struct.

`sbm-catalog plan` fills:

- `artifact_verification` with the catalog filename, size, SHA-256, and the chosen mirror or local path.
- `storage_plan.target_disk.stable_id` with the literal `unbound`.
- `storage_plan.operations` with an empty list.
- `storage_plan.destructive` with false.
- `boot_registration.register_bls_or_efi` and `set_as_default` with false.

`sbm-core` must reject a plan whose `stable_id` is `unbound` before it creates, formats, or writes a partition. The catalog will not replace that identifier. Disk identity comes from a live probe and must not be a `/dev/sdX` path.

The compatibility sidecar at `catalog/compatibility/<id>.json` is catalog-local. Core can ignore it. If core displays license, boot method, or unattended mechanism text, read the sidecar and do not expect those fields on `OsCatalogEntry` 1.0. `apple_silicon` is `unsupported` for every current entry. An `aarch64` plus `uefi-native` image is not an Apple Silicon artifact.

`user_supplied_only` artifacts have empty mirrors and, until someone imports a file, a zero size and an all-zero SHA-256. Do not fetch them. An import record in the cache is a local hash. It does not authorize redistribution.

## Agent 05 — `sbm-server`

A server may cache an image by its URL and SHA-256. It must not serve bytes whose hash differs from the catalog artifact, and it must not add a mirror for a `user_supplied_only` artifact. Windows and any other restricted image stay off the server unless a separate authorization says otherwise. This milestone does not upload images anywhere.

Catalog presentation can list `sbm-catalog list` output or the `CatalogIndex` from `index_document`. The index is a convenience document. Trust comes from the TUF targets metadata, not from an unsigned index copied off a mirror.

## `sbm-lab`

The installer boot check is `tools/boot-installer.sh`. It checks SHA-256, copies a QEMU variable store under `build/`, and starts `qemu-system-aarch64 -machine virt,accel=hvf` with the EDK2 AArch64 firmware. The CD-ROM is the installer. There is no host disk argument. The 2026-11-26 run is recorded in the research evidence note. `sbm-lab` can call the same script; the production catalog code stays in this repository.

## Offline

`export-offline` copies the catalog JSON and an optional metadata directory. It does not copy ISO files. Validation works on the exported tree with no network. A client that already holds the trusted `1.root.json` can run `verify` against a local metadata directory and a local targets directory.
