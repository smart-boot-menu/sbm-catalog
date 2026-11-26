#!/bin/sh
# Boot one already downloaded installer ISO in a disposable QEMU VM.
# This does not partition, write, or select a disk on the host.
set -eu
iso=${1:?path to the installer ISO}
expected_sha=${2:-f4e05f67b5c74e3ff7b24c2a8f889960f9eb31fe0c1ac8513d27d5656958b6b3}
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
work="$root/build"
mkdir -p "$work"
actual=$(shasum -a 256 "$iso" | awk '{print $1}')
if [ "$actual" != "$expected_sha" ]; then
  echo "sha256 $actual does not match $expected_sha" >&2
  exit 1
fi
code="/opt/homebrew/share/qemu/edk2-aarch64-code.fd"
vars_src="/opt/homebrew/share/qemu/edk2-arm-vars.fd"
qemu="/opt/homebrew/bin/qemu-system-aarch64"
for need in "$code" "$vars_src" "$qemu"; do
  if [ ! -f "$need" ] && [ ! -x "$need" ]; then
    echo "missing $need" >&2
    exit 1
  fi
done
cp "$vars_src" "$work/efivars.fd"
log="$work/boot-serial.log"
rm -f "$log"
"$qemu" \
  -machine virt,accel=hvf \
  -cpu host \
  -m 2048 \
  -smp 2 \
  -drive "if=pflash,format=raw,readonly=on,file=$code" \
  -drive "if=pflash,format=raw,file=$work/efivars.fd" \
  -cdrom "$iso" \
  -boot order=d \
  -display none \
  -serial "file:$log" \
  -monitor none \
  -no-reboot &
pid=$!
sleep 75
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
echo "serial log: $log"
