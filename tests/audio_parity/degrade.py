#!/usr/bin/env python3
"""Write a deliberately degraded copy of a render directory, to show the metrics notice it.

    degrade.py --src DIR --dst DIR --kind tempo|pitch

    tempo  5 percent faster, pitch kept (ffmpeg atempo).
    pitch  2 semitones higher, duration kept (ffmpeg asetrate, aresample, atempo).

ipa.tsv is copied unchanged. Needs ffmpeg on PATH.
"""
import argparse
import os
import shutil
import subprocess

from scipy.io import wavfile

SEMITONE = 2.0 ** (2.0 / 12.0)


def filter_for(kind, rate):
    if kind == "tempo":
        return "atempo=1.05"
    return f"asetrate={rate * SEMITONE:.3f},aresample={rate},atempo={1.0 / SEMITONE:.9f}"


def degrade(src, dst, kind):
    rate, _ = wavfile.read(src)
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-i", src, "-filter:a", filter_for(kind, rate), dst],
        check=True,
    )


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--src", required=True)
    p.add_argument("--dst", required=True)
    p.add_argument("--kind", required=True, choices=["tempo", "pitch"])
    args = p.parse_args()
    os.makedirs(args.dst, exist_ok=True)
    for name in sorted(os.listdir(args.src)):
        if name.endswith(".wav"):
            degrade(os.path.join(args.src, name), os.path.join(args.dst, name), args.kind)
    shutil.copy(os.path.join(args.src, "ipa.tsv"), os.path.join(args.dst, "ipa.tsv"))


if __name__ == "__main__":
    main()
