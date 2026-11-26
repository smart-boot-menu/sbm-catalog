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

Published metadata expires, so `repository/` is gitignored. Tests create keys in a temporary directory, publish, verify, flip a byte of `targets.json`, roll the timestamp version backward, and load an already expired timestamp. Safe expiry enforcement rejects the expired timestamp. `ExpirationEnforcement::Unsafe` loads it and exists only so the test can show the difference. `verify` uses safe enforcement unless `--allow-expired` is passed.

## Production

Production root keys stay offline. The root role threshold must be greater than one, with distinct holders. Timestamp expiry stays short so a freeze of the metadata is detected when clients refuse an expired timestamp. Snapshot binds the targets version so a mirror cannot mix a new timestamp with an old targets document. Target hashes are part of the signed targets metadata, so a mirror that changes an image or a catalog file fails verification.

Rotation is a new root version, signed by the current root threshold, that replaces role key IDs. The old root signatures remain available for the version step the client performs while it walks `N.root.json`. This repository does not ship that ceremony and does not ship production private keys.

A compromised mirror can serve stale or altered bytes. Clients that start from the trusted root and enforce version and expiry will reject a rollback, an expired timestamp, a bad signature, and a target whose hash does not match the signed metadata. They will not reject a freeze until the timestamp expires, which is why the timestamp lifetime stays short.

## What a signature does not allow

Catalog entries are data. Community contributors do not get a key that can sign the top-level targets role, and they do not get a catalog field that runs a program. Installation remains a plan with no storage operations until `sbm-core` binds a live disk identity.
