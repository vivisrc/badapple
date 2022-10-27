# Bad Apple EFI Image

Shows bad apple on your screen, boots as an EFI image. The EFI image loads a file `VIDEO.BIN` at the image's root directory. Format is raw grey values in 480x360 resolution. This is admittedly very inefficient, but I did not want to write deal with any form of compression for a project this silly. It has a pretty decent speed on my machine and is able to run at 30 FPS if it weren't for my bad frame limiter implementation.

## Building

Preparing the video requires python, compilation requires rust and cargo, and the image building script assumes a linux environment with rclone and udisks.

```sh
python -m venv ./venv
source venv/bin/activate
pip install -Ur requirements.txt
./build_video.py
cargo build --release
./image.sh release
```

## Running

The bootable image `fs0` can be flashed onto any USB that has 2GiB of space, this is an image file, not an ISO. You can also run this via QEMU with the OVMF BIOS using `qemu.sh`, The OVMF bios unfortunately doesn't seem to support the EFI timestamp protocol and thus will likely run at a speed I can't quite comprehend.
