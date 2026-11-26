# Upstream provenance

Retrieved 2026-11-26. Sizes and hashes below are the values stored in the catalog. They were taken from the named files, from HTTP HEAD `Content-Length`, or from a local digest after a published checksum matched. They were not invented to fill a field.

## Signed checksum metadata

GnuPG reported a good signature for each file below. The key was imported into an empty keyring. The untrusted-owner warning is expected. `upstream-check` repeats these checks and requires the catalog hash to be inside the signed document. A checksum that has not been through this step is not listed here.

- Ubuntu amd64 and arm64 `SHA256SUMS.gpg`, key `843938DF228D22F7B3742BC0D94AA3F0EFE21092` (`keys/upstream/ubuntu-cdimage.asc`), "Ubuntu CD Image Automatic Signing Key (2012)". amd64 SHA-256 `cc8a95cde20f6ced61a322420de00f10cc3c90ced545daa46cb9c1a117f1d927`, 2927861760 bytes. arm64 SHA-256 `af78753c94b924c85c4964c670692a853f7c80299a4064e317edad95462e3168`, 3076861952 bytes.
- Debian amd64 and arm64 `SHA256SUMS.sign`, key `DF9B9C49EAA9298432589D76DA87E80D6294BE9B` (`keys/upstream/debian-cd.asc`). amd64 SHA-256 `3770c7a54890f7574a68a8b402e553806a81d6bccfb379554caad3a9a857931e`, 794820608 bytes. arm64 SHA-256 `f4e05f67b5c74e3ff7b24c2a8f889960f9eb31fe0c1ac8513d27d5656958b6b3`, 737456128 bytes. The arm64 ISO was downloaded and its SHA-256 matched.
- Fedora 45 key `4F50A6114CD5C6976A7F1179655A4B02F577861E` (`keys/upstream/fedora-45.asc`), cleartext CHECKSUM files. x86_64 SHA-256 `a1e916f139a05955b943cf183b020780cca9c399b02b8f5ec4354c9e5c1dff1a`, 1272281088 bytes. aarch64 SHA-256 `311cb9fb35aa39b7df5768c0ccfef571a515d471e824af0d8cbe96de645ddfe8`, 1436913664 bytes. The aarch64 ISO was downloaded and `shasum -a 256` printed that same digest.
- Linux Mint `sha256sum.txt.gpg`, key `27DEB15644C6B3CF3BD7D291300F846BA25BAE09` (`keys/upstream/linux-mint-iso.asc`). Cinnamon SHA-256 `a081ab202cfda17f6924128dbd2de8b63518ac0531bcfe3f1a1b88097c459bd4`, 3091660800 bytes.
- openSUSE Leap 16.1 `.sha512.asc`, key `AD485664E901B867051AB15F35A2F86E29B700A4` (`keys/upstream/opensuse-leap.asc`), taken from the Leap 16.1 OSS `repomd.xml.key`. The signature covers the SHA-512 file. x86_64 SHA-512 `dd3605a3be177497d71b998bb608d7a276717f0f6670f53a6047067c9551cc665f38430e8e72b6ad5f6e181f0f9294502209991da738d04547a1a32a68471f89`, local SHA-256 `93aa330fddefd26aab5f79c22f581106e05ad86b9cd246fd04049ccfa181fb39`, 774897664 bytes. aarch64 SHA-512 `e324cc6b418b2586d4def9fb8eae14e354a8aa07ed58d5c56a1f2c3d894ac477603a8c73bc2cc38ced0306064de5cfe596aa74e29c56eec0ed938ba578075282`, local SHA-256 `71d9f43a0e920010526b5e1853d8add4cec1dbfdd7e5f30d132bc99da924d2f3`, 768606208 bytes.
- GParted `CHECKSUMS.TXT.gpg`, key `EB1DD5BF6F88820BBCF5356C8E94C9CD163E3FB0` (`keys/upstream/gparted-steven-shiau.asc`). SHA-256 `d789c38779f0d6f7026c12f44c2c52a04f66e28a1aea7d51f3045ad1bbf28411`, 720371712 bytes via HTTP HEAD.

## Signature present, image not checked

These URLs are real and the catalog keeps them. Automatic download is off because the signature was not verified over the ISO.

- Arch Linux 2026.11.01 `.iso.sig` is a binary image signature, key `3E80CA1A8B89F69CBA57D98A76A5EF9054449A5C` (`keys/upstream/archlinux-pierre.asc`). `sha256sums.txt.sig` returned 404. SHA-256 `01bcdcf01af2f7d58f2794899c834770d25c9bfc945bd092b32bb66672a4a216`, 1317568512 bytes. The catalog mirror is the dated URL.
- Proxmox VE 9.3-1 `.iso.asc` contains sigclass `0x00` signatures over the ISO. The first key is `24B30F06ECC1836A4E5EFECBA7BCD1420BFE778E` (`keys/upstream/proxmox-release-trixie.asc`). Verifying that signature against the checksum file was a bad signature. SHA-256 `cc4fc97b9ac3a5712ad5b665195753571923a18e11f5c2ebb113fdc29a50bd36`, 1808074752 bytes. A second signature key, `F4E136C67CDCE41AE6DE6FC81140AF8F639E0C39`, was not fetched.
- SystemRescue 13.03 `.iso.asc` is sigclass `0x00`. Signing subkey `62989046EB5C7E985ECDF5DD3B0FEA9BE13CA3C9` of primary `0FF11AF081E98345594812037091115F8320B897` (`keys/upstream/systemrescue-dupoux.asc`). Verifying it against the checksum file was a bad signature. SHA-256 `b71b98ba6b92145524cb1a72206b9888a59861ad6fb05d127faf618ca0136891`, 1434386432 bytes. The arm64 SHA-256 URL returned 404.

## Checksums without a signature

- NixOS 26.05.15079.0dc498bfe49b minimal x86_64 `68a615138a826afb6e09a8055dd6df606309cd72857f686c15256492ce83c1df`, 1744830464 bytes, and aarch64 `1bf7f6cb42c18112d026e2753baf3808035b1b0fdc4f7d4e04ec4379e03ecc5c`, 1822334976 bytes. `.iso.asc` and `.sha256.asc` returned 404. The `.sha256` objects are hashes.
- FreeBSD 15.1-RELEASE bootonly amd64 `3e74120a59512cefc35840443bdd05087c8a010a27cf8a6fbb4f7450824f092e`, 546666496 bytes, and arm64 `359136c2af73e03da6f15ad59f0c67bc561ca8b69631d78bfb8f2225e2c9a5ef`, 460095488 bytes. The 15.1 directory lists checksum files and no `.asc`. A guessed `.asc` URL returned 404. 15.2 was still BETA3 on 2026-11-20 and is not cataloged.

## Retrieval notes that are not a second verdict

NixOS channel listing used the S3 prefix `nixos-26`. `nixos-26.05/` as a directory URL was not the object that existed. Arch's catalog mirror is the dated `iso/2026.11.01/` path. FreeBSD checksum files are `CHECKSUM.SHA256-FreeBSD-15.1-RELEASE-*` under `https://download.freebsd.org/releases/ISO-IMAGES/15.1/`. SystemRescue publishes a SHA-512 next to the SHA-256; that pair is still checksum-only. Mate and Xfce hashes were in the Mint sums file and are not pinned.

## Not redistributed

Windows 11 has no mirror and no pinned hash. The catalog records Microsoft's published 4 GiB RAM and 64 GiB storage minimums as the floor. No Windows ISO was downloaded.
