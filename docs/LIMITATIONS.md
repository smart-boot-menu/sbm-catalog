# Limitations

## Booting an installer is not installing

Every current plan boots a verified image or records that a user-supplied image would be booted. The storage operation list is empty. Ubuntu autoinstall, Debian preseed, Fedora Kickstart, openSUSE AutoYaST, archinstall, NixOS installation, FreeBSD installerconfig, and Windows unattended setup are names in the compatibility profile. They are not invoked.

Windows `wim-apply` is marked experimental because applying a WIM is not the same as creating boot configuration data. That boot-configuration step is still open. The catalog does not download a Windows image.

## Architectures

x86_64 and aarch64 are recorded only where an image was pinned. Arch Linux, Linux Mint, Proxmox VE, SystemRescue, and GParted Live are x86_64 in this catalog. Ubuntu, Debian, NixOS, openSUSE, and FreeBSD aarch64 images are generic UEFI. They are not Apple Silicon `m1n1` plus U-Boot images. Resolving any of them with `--platform apple-silicon-m1n1-uboot` fails and says so when a generic UEFI aarch64 image exists.

Not pinned, though upstream publishes more: Ubuntu riscv64, ppc64el, and s390x; openSUSE ppc64le and s390x; Fedora DVD images; Arch Linux ARM; Windows on Arm; GParted i686. FreeBSD 15.2 was still BETA3 on 2026-11-20 and is not cataloged.

## Integrity

Fedora 45 Server CHECKSUM files for x86_64 and aarch64 were verified with the pinned Fedora 45 key. Debian amd64 `SHA256SUMS` was verified with Debian CD signing key `DA87E80D6294BE9B`. The Debian arm64 netinst hash matches the retrieved arm64 `SHA256SUMS`; that directory's `.sign` file was not the file the fixture test verified.

openSUSE publishes SHA-512 and a detached signature. The SHA-512 of the downloaded online installers matched. SHA-256 in the catalog is the digest of those same bytes. The `.sha512.asc` signatures were not verified.

Ubuntu, Linux Mint, Arch Linux, Proxmox VE, SystemRescue, and GParted Live have signature URLs in the catalog. Those signature files were not verified in this milestone. NixOS publishes a `.sha256` file, which is a hash and not a signature. FreeBSD publishes `CHECKSUM.SHA256-*` and the guessed `.asc` URL returned 404, so no signature URL is recorded.

A hash sitting next to an image is not authenticity. `upstream-check` fails closed when the pinned key does not verify the Fedora or Debian metadata.

## Hardware numbers

Linux, BSD, and recovery RAM and disk figures are catalog floors so a resolver can filter obvious mismatches. They are not measurements from this machine and they are not claimed as each vendor's published minimum, except Windows 11: 4096 MiB RAM and 64 GiB disk are Microsoft's published Windows 11 minimums. This session did not re-fetch that requirements page.

## Recovery images

SystemRescue and GParted Live use `direct_installer_boot` because that is the strategy for booting an image. The compatibility text says they are live environments, not operating-system installers.

## TUF

Development keys use threshold 1 and are not a production root. Metadata `spec_version` is `1.0.0` because that is what `tough` 0.24.0 signs. Timestamp metadata expires; a repository checked in today would fail safe verification later, so signed development metadata is not committed.
