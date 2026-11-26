# HandoFF — sbm-catalog 0.1

Date: 2026-11-26. Repository: `smart-boot-menu/sbm-catalog`.

## Done

- Twelve operating systems have schema 1.0 entries and compatibility profiles. `cargo test` covers validation, architecture filtering, Apple Silicon rejection, ambiguous resolve, script and private-key rejection, missing integrity, offline export, import, resumable local download, Fedora and Debian signature checks, and development TUF publish, tamper, rollback, and expiry.
- `sbm-catalog` implements `validate`, `list`, `search`, `resolve`, `download`, `verify`, `publish`, plus `plan`, `import`, `export-offline`, and `upstream-check`.
- Debian 13.8.0 arm64 netinst was downloaded, its SHA-256 matched the retrieved `SHA256SUMS`, and QEMU HVF reached the GRUB "Install" menu. Evidence is in the research repository. The plan for that image has no storage operations.
- Compatibility sidecars were used instead of forking `OsCatalogEntry`. `sbm-common` was not changed. Dependency revision is `28d38450954f0b93815e12f31da92370b13a7462`.

## Not done

- No unattended installer is executed.
- Ubuntu, Mint, Arch, Proxmox, SystemRescue, and GParted signature files are linked and not verified. openSUSE `.sha512.asc` is not verified. Debian arm64 `.sign` is linked and not the signature the fixture test checks. FreeBSD had no `.asc` at the URL that was tried.
- Windows is user-supplied only. WIM apply and BCD creation are not implemented.
- No image is offered for Apple Silicon. No production TUF root exists. Development keys use threshold 1 and are not in git. `tough` 0.24.0 signs `spec_version` `1.0.0`.
- `sbm-core` and `sbm-server` were not modified. The contracts they need are in `docs/INTEGRATION.md`.

## Interfaces

- Agent 02: read `OsCatalogEntry` and refuse `OsInstallPlan` storage plans whose `stable_id` is `unbound`.
- Agent 05: optional cache key is URL plus SHA-256. Do not cache `user_supplied_only` artifacts and do not serve bytes that miss the catalog hash.
- `sbm-lab`: `tools/boot-installer.sh` plus the 2026-11-26 evidence note. The script has no host disk argument.

## Next

Verify the remaining upstream signatures, or stop publishing their signature URLs until that happens. Bind a lab disk inside `sbm-core` before any automated install. Keep the production root offline and raise its threshold before a public repository is signed.
