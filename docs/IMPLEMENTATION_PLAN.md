# Implementation plan

This is the plan followed for the 0.1 catalog milestone. It is a record of the decisions, not a claim that every upstream installer is automated.

## Schema

`OsCatalogEntry` schema 1.0 stays the wire contract with `sbm-core`. The milestone does not fork that struct and does not regenerate `sbm-common` schemas.

Fields the wire type does not carry — license, per-file architecture, image format, boot method, checksum and signature URLs, Apple Silicon explicitness, and unattended mechanism names — go in `catalog/compatibility/<id>.json`. Serde already ignores unknown fields on the wire type, so a future schema can absorb the sidecar without breaking readers that only know 1.0.

A release lists a platform only when every artifact in that release can boot that platform. Ubuntu, Debian, NixOS, openSUSE Leap, and FreeBSD therefore split x86_64 and aarch64 into two releases that share a version string. Fedora 45 Server netinst is one release because both architectures are UEFI-only.

`apple-silicon-m1n1-uboot` is accepted only when the compatibility profile says `explicit-m1n1-uboot-artifact` and the release lists `aarch64`. Generic `aarch64` does not satisfy that check.

## What is executable

The Rust crate validates entries, resolves one image, downloads over HTTPS with resume, checks size and SHA-256, imports a user-supplied file, writes an install plan with no disk operations, exports the JSON catalog for offline use, and signs a development TUF repository with `tough` 0.24.0.

Unattended mechanism names are data. Nothing in the catalog is a shell script, and the validator rejects shebangs and private-key material in the tree.

## What is recorded and not executed

Ubuntu autoinstall, Debian preseed, Fedora Kickstart, and openSUSE AutoYaST are marked `stable` because those projects publish them as installer interfaces. Arch's `archinstall`, NixOS's declarative install, FreeBSD `installerconfig`, and Windows `autounattend` / WIM apply are marked `experimental` or left unresolved where the boot-configuration step is still open. SystemRescue and GParted Live are recovery images; their strategy name boots the live image.

## Supply chain

A checksum downloaded beside an image is not treated as proof. Fedora 45 CHECKSUM files are cleartext-signed and checked with the pinned Fedora 45 key. Debian amd64 `SHA256SUMS` is checked with the pinned Debian CD signing key. openSUSE SHA-256 values were computed locally after the bytes matched the published SHA-512. Other signature URLs are stored and called unverified in the provenance note.

Windows has no mirrors and an unpinned hash. `download` refuses that artifact.

## Lab

The first boot demonstration is Debian 13.8.0 arm64 netinst under `qemu-system-aarch64` with HVF and EDK2. That virtual machine is generic UEFI, not Apple Silicon. The host boot configuration is not modified.
