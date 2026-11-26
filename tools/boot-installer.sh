#!/bin/sh
# Boot one already downloaded installer ISO in a disposable QEMU VM.
# Arguments: ISO path, expected sha256, optional seconds to wait before SIGTERM.
# This does not partition, write, or select a disk on the host.
set -eu
iso=${1:?path to the installer ISO}
expected_sha=${2:?expected sha256}
wait_seconds=${3:-75}
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
work="$root/build"
mkdir -p "$work"
case "$iso" in
  /dev/*) echo "refusing a host device path" >&2; exit 1 ;;
esac
actual=$(shasum -a 256 "$iso" | awk '{print $1}')
if [ "$actual" != "$expected_sha" ]; then
  echo "sha256 $actual does not match $expected_sha" >&2
  exit 1
fi
code="/opt/homebrew/share/qemu/edk2-aarch64-code.fd"
vars_src="/opt/homebrew/share/qemu/edk2-arm-vars.fd"
qemu="/opt/homebrew/bin/qemu-system-aarch64"
for need in "$code" "$vars_src" "$qemu"; do
  if [ ! -e "$need" ]; then
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
sleep "$wait_seconds"
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
echo "serial log: $log"
