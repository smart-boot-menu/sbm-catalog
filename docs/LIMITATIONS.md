# Limitations

## Booting an installer is not installing

Every current plan boots an image or records that a user-supplied image would be booted. The storage operation list is empty. Ubuntu autoinstall, Debian preseed, and Fedora Kickstart have declarative recipes under `catalog/recipes/`. Those files set `executed` to false, omit disk layouts, and reject command, `%post`, and shell text. They are not applied. openSUSE AutoYaST, archinstall, NixOS installation, FreeBSD installerconfig, and Windows unattended setup remain names in the compatibility profile.

Windows `wim-apply` is marked experimental because applying a WIM is not the same as creating boot configuration data. That boot-configuration step is still open. The catalog does not download a Windows image.

## Architectures

x86_64 and aarch64 are recorded only where an image was pinned. Arch Linux, Linux Mint, Proxmox VE, SystemRescue, and GParted Live are x86_64 in this catalog. Ubuntu, Debian, NixOS, openSUSE, and FreeBSD aarch64 images are generic UEFI. They are not Apple Silicon `m1n1` plus U-Boot images. Resolving any of them with `--platform apple-silicon-m1n1-uboot` fails and says so when a generic UEFI aarch64 image exists.

Not pinned, though upstream publishes more: Ubuntu riscv64, ppc64el, and s390x; openSUSE ppc64le and s390x; Fedora DVD images; Arch Linux ARM; Windows on Arm; GParted i686. FreeBSD 15.2 was still BETA3 on 2026-11-20 and is not cataloged.

## Integrity

`authenticity` on each compatibility artifact is one of `signed-metadata`, `signed-image`, `unverified-signature`, `checksum-only`, or `user-supplied`. Automatic download runs only for the first two. `signed-image` is reserved for a signature that has been checked against the ISO itself. No current entry uses it.

`signed-metadata` means a pinned key verified a checksum document and the catalog hash is inside that document. That covers Ubuntu, Debian, Fedora, Linux Mint, openSUSE Leap, and GParted Live. openSUSE's signature covers the SHA-512 file. The catalog SHA-256 is a local digest taken after the downloaded bytes matched that SHA-512. `upstream-check` re-verifies the signed SHA-512. It does not download the ISO again.

`unverified-signature` means a signature URL exists and the signed payload was not checked against the image. Arch Linux, Proxmox VE, and SystemRescue are in this class. Their signature packets are over the ISO (sigclass `0x00`). Checking those signatures against the checksum file produced a bad signature for Proxmox and SystemRescue, which confirms the signature does not cover the checksum. It does not by itself say the hash is wrong. Automatic download is disabled.

`checksum-only` means no usable signature was found. NixOS and FreeBSD are in this class. A hash from the same place as the image is not upstream authentication. Automatic download is disabled unless the caller passes `--allow-unauthenticated`.

Windows is `user-supplied`. Download is refused. `--allow-unauthenticated` does not override that.

`upstream-check` verifies every `signed-metadata` fixture. A changed checksum document fails. Interrupted downloads keep `cache/<filename>.partial.json` with the expected size and SHA-256. A journal that does not match deletes the partial and starts over. `--offline` returns a complete verified cache file and does not open the network.

## Hardware numbers

Linux, BSD, and recovery RAM and disk figures are catalog floors so a resolver can filter obvious mismatches. They are not measurements from this machine and they are not claimed as each vendor's published minimum, except Windows 11: 4096 MiB RAM and 64 GiB disk are Microsoft's published Windows 11 minimums. This session did not re-fetch that requirements page.

## Recovery images

SystemRescue and GParted Live use `direct_installer_boot` because that is the strategy for booting an image. The compatibility text says they are live environments, not operating-system installers.

## TUF

Development keys use threshold 1 and are not a production root. Metadata `spec_version` is `1.0.0` because that is what `tough` 0.24.0 signs. Timestamp metadata expires; a repository checked in today would fail safe verification later, so signed development metadata is not committed.
