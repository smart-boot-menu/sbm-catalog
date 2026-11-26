# Handoff — sbm-catalog 0.1

Date: 2026-11-26. Repository: `smart-boot-menu/sbm-catalog`.

## Verification

Automatic download is allowed only for `signed-metadata` and `signed-image`. A checksum published beside an image is not that proof. `sbm-catalog download` refuses `checksum-only`, `unverified-signature`, and `user-supplied` unless the caller passes `--allow-unauthenticated`. Windows stays user-supplied and is refused even with that flag.

`upstream-check` re-verifies every `signed-metadata` fixture with the pinned public key and requires the catalog hash to appear in the signed document. A flipped byte in the Ubuntu amd64 `SHA256SUMS` fails. GnuPG's untrusted-owner warning is expected.

| Entry | Class | What was checked on 2026-11-26 |
| --- | --- | --- |
| Ubuntu 26.04.1 amd64 and arm64 | `signed-metadata` | `SHA256SUMS.gpg`, Ubuntu CD Image Automatic Signing Key `843938DF228D22F7B3742BC0D94AA3F0EFE21092` |
| Debian 13.8.0 amd64 and arm64 | `signed-metadata` | `SHA256SUMS.sign`, Debian CD signing key `DF9B9C49EAA9298432589D76DA87E80D6294BE9B` |
| Fedora 45 Server netinst, both arches | `signed-metadata` | cleartext `CHECKSUM`, Fedora 45 key `4F50A6114CD5C6976A7F1179655A4B02F577861E` |
| Linux Mint 22.3 Cinnamon | `signed-metadata` | `sha256sum.txt.gpg`, Linux Mint ISO Signing Key `27DEB15644C6B3CF3BD7D291300F846BA25BAE09` |
| openSUSE Leap 16.1 online, both arches | `signed-metadata` | `.sha512.asc` covers the SHA-512 file, openSUSE Project Signing Key `AD485664E901B867051AB15F35A2F86E29B700A4`. Catalog SHA-256 is a local digest of bytes that matched that SHA-512 |
| GParted Live 1.8.1-6 | `signed-metadata` | `CHECKSUMS.TXT.gpg`, Steven Shiau `EB1DD5BF6F88820BBCF5356C8E94C9CD163E3FB0` |
| Arch Linux 2026.11.01 | `unverified-signature` | `.sig` is an image signature from `3E80CA1A8B89F69CBA57D98A76A5EF9054449A5C`. The ISO was not verified |
| Proxmox VE 9.3-1 | `unverified-signature` | `.asc` is sigclass `0x00` over the ISO. Checking it against the checksum file was a bad signature. The ISO was not verified. First key `24B30F06ECC1836A4E5EFECBA7BCD1420BFE778E` |
| SystemRescue 13.03 | `unverified-signature` | `.asc` is sigclass `0x00`. Checking it against the checksum file was a bad signature. Signing subkey `62989046EB5C7E985ECDF5DD3B0FEA9BE13CA3C9`. The ISO was not verified |
| NixOS 26.05.15079.0dc498bfe49b | `checksum-only` | `.iso.asc` and `.sha256.asc` returned 404. The `.sha256` object is a hash |
| FreeBSD 15.1-RELEASE | `checksum-only` | the ISO directory has `CHECKSUM.SHA256` and `CHECKSUM.SHA512` and no signature |
| Windows 11 | `user-supplied` | no mirror, no pinned hash, no image in this repository |

A bad signature against a checksum file means the signature does not cover that file. It does not mean the catalog hash is wrong.

`cargo test` is 22 tests. They cover the fixture manifest, Ubuntu tamper, architecture and firmware filtering, recipe refusal, partial-journal restart, offline cache, TUF metadata tamper, timestamp rollback, snapshot mix-and-match, target-byte tamper, expired timestamp, and a fresh datastore presented with an already expired timestamp.

## Installer boots

These are boot logs, not benchmarks. No host disk was attached.

- Debian 13.8.0 arm64 netinst. QEMU HVF, SIGTERM after 75 seconds, serial 6769 bytes, GRUB `*Install` and `Graphical install`. Research note `validation/evidence/2026-11-26-debian-13.8.0-arm64-netinst-qemu.md`.
- openSUSE Leap 16.1 aarch64 online installer. SHA-256 checked before boot. Serial 31633 bytes. GRUB 2.14 showed `Install Leap 16.1 (aarch64)`. The menu timeout started the guest kernel. The log reached initramfs unpack. No disk was present and no installation finished.
- Fedora 45 Server aarch64 netinst. SHA-256 `311cb9fb35aa39b7df5768c0ccfef571a515d471e824af0d8cbe96de645ddfe8` matched the signed CHECKSUM and the boot script. Serial 33022 bytes. GRUB 2.12 showed `Install Fedora 45` and `Test this media & install Fedora 45`. The highlighted entry started the installer kernel with `rd.live.check`. No disk was present and no installation finished.

`tools/boot-installer.sh` takes an ISO path, the expected SHA-256, and a wait in seconds. It refuses a `/dev/` path.

## Integration

- `sbm-core` `sbm_runtime::inspect_catalog_plan` compares `OsInstallPlan` with `OsCatalogEntry`. A matching unbound plan is accepted with `execution_enabled` false. A hash mismatch is `SignatureInvalid`. Storage operations on `stable_id` `unbound` are `DiskIdentityMismatch`. SBM 0.1 still does not execute disk changes.
- `sbm-lab` does not attach installer ISOs itself. `docs/installer-boot.md` in that repository points at `tools/boot-installer.sh` and the research notes. The lab harness still has no CD-ROM flag and must not gain a host-disk path for this.
- `sbm-catalog plan` prints the canonical plan plus the declarative recipe when one exists. Ubuntu, Debian, and Fedora recipes set `executed` to false. Storage answers are omitted. The plan's operation list stays empty.
- `list --arch` and `list --platform` drop images that do not match. Legacy BIOS drops Fedora. aarch64 drops Arch, Mint, Proxmox, SystemRescue, and GParted.

## Development TUF

`tools/publish-dev.sh` validates, runs `upstream-check`, publishes with development keys under `build/dev-keys`, and verifies the result. Private keys stay in that directory. `repository/` stays gitignored because timestamp metadata expires. Production root keys were not created. The ceremony is in `docs/TUF.md`.

## Blockers

- Arch, Proxmox, and SystemRescue stay download-disabled until `gpg --verify` succeeds against the ISO itself. That has not been done.
- NixOS and FreeBSD stay checksum-only until an upstream signature exists and is checked.
- Windows stays user-supplied. No Windows image is redistributed. WIM apply and BCD creation are not implemented.
- No unattended install is executed. Recipes are data.
- No image is offered for Apple Silicon `m1n1` plus U-Boot.
- No production TUF root exists. Development threshold is 1. `tough` 0.24.0 signs `spec_version` `1.0.0`.
- `sbm-core` does not yet download catalog images or bind a live `DiskIdentity`. An unbound plan cannot become executable inside 0.1.
- openSUSE catalog SHA-256 is a local digest. The signed document authenticates SHA-512. Re-hash the ISO to reconfirm SHA-256; the fixture test does not download the ISO.
