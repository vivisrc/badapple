#!/bin/sh

qemu-system-x86_64 -enable-kvm -bios /usr/share/OVMF/OVMF_CODE.fd -drive file=./fs0.img,format=raw
