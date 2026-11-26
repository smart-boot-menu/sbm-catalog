#!/bin/sh
# Sign a development TUF repository. Private keys stay in the key directory.
# Usage: tools/publish-dev.sh [key-dir] [output-dir]
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$root"
keys=${1:-build/dev-keys}
out=${2:-build/repository}
case "$keys" in
  "$root"/catalog|"$root"/catalog/*|"$root"/keys|"$root"/keys/*)
    echo "refusing to place signing keys inside the catalog or the public key directory" >&2
    exit 1
    ;;
esac
case "$out" in
  "$root"/catalog|"$root"/catalog/*)
    echo "refusing to write repository metadata inside the catalog tree" >&2
    exit 1
    ;;
esac
cargo run --quiet -- --catalog catalog validate
cargo run --quiet -- --catalog catalog upstream-check
cargo run --quiet -- --catalog catalog publish --keys "$keys" --out "$out" --version 1
cargo run --quiet -- --catalog catalog verify \
  --metadata "$out/metadata" \
  --targets "$out/targets" \
  --datastore "$out/datastore"
echo "development repository is in $out"
echo "private keys stay in $keys and are not committed"
