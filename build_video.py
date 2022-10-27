#!/usr/bin/env python


import os
from tempfile import mktemp

from cv2 import VideoCapture
from yt_dlp import YoutubeDL


def main():
    video_path = mktemp(suffix=".mp4", prefix="badapple.")
    with YoutubeDL(
        {
            "format": "h264_360p_low-aac_64kbps-hls",
            "outtmpl": video_path,
        }
    ) as ytdl:
        ytdl.download(["https://www.nicovideo.jp/watch/sm8628149"])

    print("source video downloaded, building video.bin.", end="", flush=True)

    os.makedirs("esp", exist_ok=True)
    with open("esp/VIDEO.BIN", "wb") as f:
        cap = VideoCapture(video_path)
        ret, frame = cap.read()
        while ret:
            f.write(
                bytearray(
                    sum(pixel) // len(pixel) for row in frame for pixel in row
                )
            )
            print(".", end="", flush=True)
            ret, frame = cap.read()
        cap.release()

    os.remove(video_path)
    print("\ndone.")


if __name__ == "__main__":
    main()
