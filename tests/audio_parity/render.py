#!/usr/bin/env python3
"""Render every corpus row to a WAV with one engine, plus its IPA and -x strings.

    render.py --engine c|rust --out DIR [--corpus corpus.tsv] [--jobs 8]
              [--c-bin PATH --c-data DIR] [--rust-bin PATH --rust-data DIR]

Writes DIR/<id>.wav and DIR/ipa.tsv (id, --ipa output, -x output). <id> is the
1-based row number in the corpus, zero-padded to four digits. Both engines run
with the same explicit voice, speed, pitch and amplitude flags.
"""
import argparse
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
SPEECH_FLAGS = ["-s", "175", "-p", "50", "-a", "100"]


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("--engine", required=True, choices=["c", "rust"])
    p.add_argument("--out", required=True)
    p.add_argument("--corpus", default=os.path.join(HERE, "corpus.tsv"))
    p.add_argument("--jobs", type=int, default=8)
    p.add_argument("--c-bin", default=os.environ.get("C_ESPEAK_BIN"))
    p.add_argument("--c-data", default=os.environ.get("C_ESPEAK_DATA"))
    p.add_argument("--rust-bin", default=os.path.join(ROOT, "target", "release", "espeak-ng-rs"))
    p.add_argument("--rust-data", default=os.path.join(ROOT, "espeak-ng-data"))
    return p.parse_args()


def read_corpus(path):
    with open(path, encoding="utf-8") as f:
        return [line.split("\t", 1) for line in f.read().splitlines() if line]


def run(binary, data, args, text):
    env = dict(os.environ, ESPEAK_DATA_PATH=data)
    done = subprocess.run([binary, *args, "--", text], capture_output=True, env=env, timeout=60)
    return done.returncode, done.stdout.decode("utf-8", "replace").strip().replace("\n", " ⏎ ")


def render_row(job):
    binary, data, out, row_id, lang, text = job
    wav = os.path.join(out, row_id + ".wav")
    code, _ = run(binary, data, ["-v", lang, *SPEECH_FLAGS, "-w", wav], text)
    _, ipa = run(binary, data, ["-q", "--ipa", "-v", lang], text)
    _, mnemonics = run(binary, data, ["-q", "-x", "-v", lang], text)
    return row_id, code, ipa, mnemonics


def main():
    a = parse_args()
    binary, data = (a.c_bin, a.c_data) if a.engine == "c" else (a.rust_bin, a.rust_data)
    if not binary or not data:
        sys.exit("--c-bin/--c-data (or C_ESPEAK_BIN/C_ESPEAK_DATA) are required for --engine c")
    os.makedirs(a.out, exist_ok=True)
    rows = read_corpus(a.corpus)
    jobs = [(binary, data, a.out, f"{i:04d}", lang, text) for i, (lang, text) in enumerate(rows, 1)]
    with ThreadPoolExecutor(a.jobs) as pool:
        results = list(pool.map(render_row, jobs))
    with open(os.path.join(a.out, "ipa.tsv"), "w", encoding="utf-8") as f:
        for row_id, _, ipa, mnemonics in results:
            f.write(f"{row_id}\t{ipa}\t{mnemonics}\n")
    failed = [r[0] for r in results if r[1] != 0]
    print(f"{a.engine}: {len(results)} rows rendered, {len(failed)} failed {failed[:10]}")


if __name__ == "__main__":
    main()
