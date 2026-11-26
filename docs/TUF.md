# Catalog TUF publishing

The catalog is distributed as a TUF repository. The implementation uses `tough` 0.24.0, the crate named in the Smart Boot Menu technology stack. `tough` writes metadata `spec_version` `1.0.0`. That is the string its editor signs. The client behavior covered here is the TUF 1.0 workflow the architecture asked for: signed root, targets, snapshot, and timestamp metadata, version rollback rejection, timestamp expiry, threshold signatures, and target hashes. Delegated roles are created with the editor and were loaded by the 0.24 client in `development_tuf_rejects_tampering_rollback_and_expiry`.

## Development keys

`sbm-catalog publish --keys build/dev-keys --out build/repository` creates five Ed25519 keys when `1.root.json` is absent:

| File | Role |
| --- | --- |
| `root.pkcs8` | root |
| `targets.pkcs8` | targets |
| `snapshot.pkcs8` | snapshot |
| `timestamp.pkcs8` | timestamp |
| `community.pkcs8` | delegated `community` role only |

The files are PKCS#8 DER. `tough` 0.24 parses raw PKCS#8 for Ed25519; its PEM `"PRIVATE KEY"` path accepts RSA only. `*.pkcs8` and `*.pem` are gitignored. Do not commit them. Do not point `--keys` or `--out` at the `catalog/` tree; publish refuses those paths.

Development thresholds are 1. Root expiry is about one year from key creation. Targets last 30 days, snapshot 7 days, and timestamp 1 day. `consistent_snapshot` is false, so clients fetch `timestamp.json`, `snapshot.json`, `targets.json`, and the versioned `1.root.json`. The trusted root passed to the client is that `1.root.json`.

The `community` role is terminating and its path set is `community/*`. It cannot sign root, snapshot, timestamp, or the top-level targets role. A signature from that role does not relax the catalog validator: a shebang or a private key in `catalog/` is still rejected.

Published metadata expires, so `repository/` is gitignored. Tests create keys in a temporary directory and then reject a flipped `targets.json`, a timestamp rolled backward, a version-2 snapshot replaced by the version-1 snapshot, a flipped target file, an expired timestamp, and a fresh datastore given only an already expired timestamp. Safe expiry enforcement rejects the expired timestamp. `ExpirationEnforcement::Unsafe` loads it and exists only so the test can show the difference. `verify` uses safe enforcement unless `--allow-expired` is passed.

`tools/publish-dev.sh` is the development publication path. It refuses a key directory inside `catalog/` or `keys/`, runs `validate` and `upstream-check`, publishes version 1 into `build/repository`, and verifies that repository. The script prints that the private keys stay in the key directory.

## Production ceremony

This milestone does not generate production keys and does not commit any private key. The development keys above are not a stand-in for the following ceremony.

1. Generate the root key on an air-gapped machine. Keep that machine offline for the life of the key. Do not copy the root private key into this repository, into CI, or into the machine that signs daily timestamp metadata.
2. Set the root threshold above one. Give each signature to a different holder. A single laptop must not be able to replace the root.
3. Generate targets, snapshot, and timestamp keys separately from the root. The timestamp signer can be online. Its lifetime stays short, on the order of a day, so a freeze of the metadata fails once that timestamp expires. Snapshot lifetime stays longer than timestamp and shorter than targets. Snapshot records the targets version, so a mirror cannot mix a new timestamp with an old targets document.
4. The first root metadata is signed by the root threshold and becomes the trusted root clients already have. Later rotation writes a new root version, signed by the current threshold, that names the replacement key IDs. Clients walk `N.root.json` using the signatures they already trust. Retire a holder by publishing that new root, not by deleting an old file from a mirror.
5. Target hashes live in the signed targets metadata. A mirror that changes a catalog file or an image fails verification even when the timestamp is current. The terminating `community` role stays limited to `community/*` and does not sign root, snapshot, timestamp, or top-level targets.
6. Store the air-gapped root private keys outside this git repository. Public keys that clients need can be published. Development PKCS#8 files under `build/dev-keys` are gitignored and are not those production keys.

A compromised mirror can serve stale or altered bytes. Clients that start from the trusted root and enforce version and expiry will reject a rollback, an expired timestamp, a bad signature, and a target whose hash does not match the signed metadata. They will not reject a freeze until the timestamp expires, which is why the timestamp lifetime stays short.

## What a signature does not allow

Catalog entries are data. Community contributors do not get a key that can sign the top-level targets role, and they do not get a catalog field that runs a program. Installation remains a plan with no storage operations until `sbm-core` binds a live disk identity.
