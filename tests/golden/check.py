#!/usr/bin/env python3
"""Golden-corpus ratchet.

Runs the Rust release binary over tests/golden/corpus/*.txt and counts the rows
that differ from tests/golden/baseline/*.tsv (C espeak-ng output). Fails if any
language exceeds its ceiling in tests/golden/ceilings.json.

    python3 -I tests/golden/check.py [--binary PATH] [--data DIR] [--out DIR]
                                     [--write-ceilings] [--show N]

--write-ceilings rewrites ceilings.json with the measured counts. Only use it to
lower ceilings; a rise means a regression.
"""
import json
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))


def arg(name, default):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


BIN = arg("--binary", os.path.join(ROOT, "target", "release", "espeak-ng-rs"))
DATA = arg("--data", os.path.join(ROOT, "espeak-ng-data"))
OUT = arg("--out", None)
SHOW = int(arg("--show", "0"))
CEILINGS = os.path.join(HERE, "ceilings.json")
ENV = dict(os.environ, ESPEAK_DATA_PATH=DATA)


def run(args, text):
    r = subprocess.run([BIN, *args, "--", text], capture_output=True, env=ENV, timeout=30)
    return r.stdout.decode("utf-8", "replace").strip().replace("\n", " ⏎ ").replace("\t", " ") + (
        f" [rc={r.returncode}]" if r.returncode else "")


def one(job):
    lang, text = job
    return text + "\t" + run(["-q", "--ipa", "-v", lang], text) + "\t" + run(["-q", "-x", "-v", lang], text)


def capture(lang):
    path = os.path.join(HERE, "corpus", lang + ".txt")
    with open(path, encoding="utf-8") as f:
        lines = f.read().splitlines()
    with ThreadPoolExecutor(8) as ex:
        return list(ex.map(one, [(lang, line) for line in lines]))


def baseline(lang):
    with open(os.path.join(HERE, "baseline", lang + ".tsv"), encoding="utf-8") as f:
        return f.read().splitlines()


def count_diffs(base, rows):
    diffs = [(a, b) for a, b in zip(base, rows) if a != b]
    return len(diffs) + abs(len(base) - len(rows)), diffs


def main():
    langs = sorted(fn[:-4] for fn in os.listdir(os.path.join(HERE, "corpus")) if fn.endswith(".txt"))
    measured = {}
    for lang in langs:
        rows = capture(lang)
        if OUT:
            os.makedirs(OUT, exist_ok=True)
            with open(os.path.join(OUT, lang + ".tsv"), "w", encoding="utf-8") as f:
                f.write("\n".join(rows) + "\n")
        n, diffs = count_diffs(baseline(lang), rows)
        measured[lang] = n
        for a, b in diffs[:SHOW]:
            print(f"   {lang} {a.split(chr(9))[0]!r}: {a.split(chr(9))[1]} -> {b.split(chr(9))[1]}")

    if "--write-ceilings" in sys.argv:
        with open(CEILINGS, "w", encoding="utf-8") as f:
            json.dump(measured, f, indent=2, sort_keys=True)
            f.write("\n")
        print("ceilings written")
        return 0

    with open(CEILINGS, encoding="utf-8") as f:
        ceilings = json.load(f)
    failed = False
    for lang in langs:
        ceil = ceilings.get(lang)
        n = measured[lang]
        status = "OK"
        if ceil is None:
            status, failed = "NO CEILING", True
        elif n > ceil:
            status, failed = "FAIL", True
        elif n < ceil:
            status = "LOWER THE CEILING"
        print(f"{lang:4} {n:6} diffs  ceiling {ceil}  {status}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
