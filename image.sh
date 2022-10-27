#!/bin/sh

set -e

release="${1:?release profile not specified, run as \'"$0" debug\' or \'"$0" release\'}"

if ! test -f ./fs0.img; then
    fallocate -v -l 2GB ./fs0.img
    mkfs -V -t fat -F 32 ./fs0.img
fi

loop="$(udisksctl loop-setup -f ./fs0.img | tee /dev/stderr | grep -oP '/dev/loop[0-9]+')"
mountpoint="$(udisksctl mount -b "$loop" -o rw | tee /dev/stderr | sed -E 's/.* at ([^ ]+).*/\1/')"

rclone --no-update-modtime -v sync ./esp/ "$mountpoint"
mkdir -pv "$mountpoint"/EFI/BOOT
cp -v ./target/x86_64-unknown-uefi/"$release"/badapple.efi "$mountpoint"/EFI/BOOT/BOOTX64.EFI

udisksctl unmount -b "$loop"
udisksctl loop-delete -b "$loop"
