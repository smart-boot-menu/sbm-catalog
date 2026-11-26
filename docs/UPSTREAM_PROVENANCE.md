# Upstream provenance

Retrieved 2026-11-26. Sizes and hashes below are the values stored in the catalog. They were taken from the named files, from HTTP HEAD `Content-Length`, or from a local digest after a published checksum matched. They were not invented to fill a field.

## Signature checks performed

- Fedora 45 key `4F50A6114CD5C6976A7F1179655A4B02F577861E` (`keys/upstream/fedora-45.asc`), cleartext `Fedora-Server-45-1.4-x86_64-CHECKSUM` and `Fedora-Server-45-1.4-aarch64-CHECKSUM`. GnuPG reported a good signature. The untrusted-owner warning is expected for a key that is pinned rather than certified in a personal web of trust. The automated test repeats this check.
- Debian CD signing key `DF9B9C49EAA9298432589D76DA87E80D6294BE9B` (`keys/upstream/debian-cd.asc`) over `SHA256SUMS` for `debian-13.8.0-amd64-netinst.iso`. GnuPG reported a good signature. The automated test repeats this check and rejects a flipped byte.
- Debian arm64 netinst was downloaded from `https://cdimage.debian.org/debian-cd/13.8.0/arm64/iso-cd/debian-13.8.0-arm64-netinst.iso` (737456128 bytes). Its SHA-256 is `f4e05f67b5c74e3ff7b24c2a8f889960f9eb31fe0c1ac8513d27d5656958b6b3`, matching the retrieved arm64 `SHA256SUMS`.
- openSUSE Leap 16.1 online installers were downloaded. Local SHA-512 matched the published `.sha512` files, and local SHA-256 was then recorded:
  - x86_64 `93aa330fddefd26aab5f79c22f581106e05ad86b9cd246fd04049ccfa181fb39`, 774897664 bytes, SHA-512 `dd3605a3be177497d71b998bb608d7a276717f0f6670f53a6047067c9551cc665f38430e8e72b6ad5f6e181f0f9294502209991da738d04547a1a32a68471f89`
  - aarch64 `71d9f43a0e920010526b5e1853d8add4cec1dbfdd7e5f30d132bc99da924d2f3`, 768606208 bytes, SHA-512 `e324cc6b418b2586d4def9fb8eae14e354a8aa07ed58d5c56a1f2c3d894ac477603a8c73bc2cc38ced0306064de5cfe596aa74e29c56eec0ed938ba578075282`

## Hashes taken from upstream metadata without a signature check in this milestone

- Ubuntu 26.04.1 live server amd64 `cc8a95cde20f6ced61a322420de00f10cc3c90ced545daa46cb9c1a117f1d927`, 2927861760 bytes, from `https://releases.ubuntu.com/26.04/SHA256SUMS`. `SHA256SUMS.gpg` was downloaded (833 bytes) and not verified.
- Ubuntu 26.04.1 live server arm64 `af78753c94b924c85c4964c670692a853f7c80299a4064e317edad95462e3168`, 3076861952 bytes, from the cdimage `SHA256SUMS` for `releases/26.04/release`.
- Arch Linux 2026.11.01 `01bcdcf01af2f7d58f2794899c834770d25c9bfc945bd092b32bb66672a4a216`, 1317568512 bytes, from `https://geo.mirror.pkgbuild.com/iso/latest/sha256sums.txt` and the dated path `iso/2026.11.01/`. The `.sig` file is about 141 bytes and was not verified. The catalog mirror is the dated URL.
- Linux Mint 22.3 Cinnamon `a081ab202cfda17f6924128dbd2de8b63518ac0531bcfe3f1a1b88097c459bd4`, 3091660800 bytes, from `https://mirrors.edge.kernel.org/linuxmint/stable/22.3/sha256sum.txt`.
- NixOS 26.05.15079.0dc498bfe49b minimal x86_64 `68a615138a826afb6e09a8055dd6df606309cd72857f686c15256492ce83c1df`, 1744830464 bytes, and aarch64 `1bf7f6cb42c18112d026e2753baf3808035b1b0fdc4f7d4e04ec4379e03ecc5c`, 1822334976 bytes. The `.sha256` objects are hashes. Channel listing used the NixOS S3 prefix `nixos-26`; `nixos-26.05/` as a directory URL was not the object that existed.
- FreeBSD 15.1-RELEASE bootonly amd64 `3e74120a59512cefc35840443bdd05087c8a010a27cf8a6fbb4f7450824f092e`, 546666496 bytes, and arm64 `359136c2af73e03da6f15ad59f0c67bc561ca8b69631d78bfb8f2225e2c9a5ef`, 460095488 bytes, from `CHECKSUM.SHA256-FreeBSD-15.1-RELEASE-*` under `https://download.freebsd.org/releases/ISO-IMAGES/15.1/`. `CHECKSUM.SHA256-FreeBSD-15.1-RELEASE-amd64.asc` returned 404.
- Proxmox VE 9.3-1 `cc4fc97b9ac3a5712ad5b665195753571923a18e11f5c2ebb113fdc29a50bd36`, 1808074752 bytes, from the download page and `https://enterprise.proxmox.com/iso/proxmox-ve_9.3-1.iso.sha256`.
- SystemRescue 13.03 amd64 `b71b98ba6b92145524cb1a72206b9888a59861ad6fb05d127faf618ca0136891`, 1434386432 bytes, and the published SHA-512, from `https://www.system-rescue.org/releases/13.03/`. The arm64 SHA-256 URL returned 404.
- GParted Live 1.8.1-6 amd64 `d789c38779f0d6f7026c12f44c2c52a04f66e28a1aea7d51f3045ad1bbf28411`, 720371712 bytes via HTTP HEAD, SHA-256 and SHA-512 from `https://gparted.org/gparted-live/stable/CHECKSUMS.TXT`.

## Not redistributed

Windows 11 has no mirror and no pinned hash. The catalog records Microsoft's published 4 GiB RAM and 64 GiB storage minimums as the floor. No Windows ISO was downloaded.
