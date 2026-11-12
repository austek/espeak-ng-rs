#!/usr/bin/env python3
"""Capture C espeak-ng output for tests/golden/corpus into tests/golden/baseline.

    python3 -I tests/golden/capture.py --c-bin PATH --c-data DIR [--langs a,b] [--force]

--c-bin is the C espeak-ng binary, --c-data the directory that holds its
espeak-ng-data (ESPEAK_DATA_PATH). Languages that already have a baseline are
skipped unless --force is given. Row format matches check.py.
"""
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))


def arg(name, default):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


BIN = arg("--c-bin", None)
ENV = dict(os.environ, ESPEAK_DATA_PATH=arg("--c-data", ""))


def run(args, text):
    r = subprocess.run([BIN, *args, "--", text], capture_output=True, env=ENV, timeout=30)
    return r.stdout.decode("utf-8", "replace").strip().replace("\n", " ⏎ ").replace("\t", " ") + (
        f" [rc={r.returncode}]" if r.returncode else "")


def one(job):
    lang, text = job
    return text + "\t" + run(["-q", "--ipa", "-v", lang], text) + "\t" + run(["-q", "-x", "-v", lang], text)


def main():
    if not BIN:
        sys.exit("--c-bin and --c-data required")
    only = arg("--langs", "")
    wanted = set(only.split(",")) if only else None
    os.makedirs(os.path.join(HERE, "baseline"), exist_ok=True)
    for fn in sorted(os.listdir(os.path.join(HERE, "corpus"))):
        lang = fn[:-4]
        out = os.path.join(HERE, "baseline", lang + ".tsv")
        if not fn.endswith(".txt") or (wanted and lang not in wanted):
            continue
        if os.path.exists(out) and "--force" not in sys.argv:
            continue
        with open(os.path.join(HERE, "corpus", fn), encoding="utf-8") as f:
            lines = f.read().splitlines()
        with ThreadPoolExecutor(8) as ex:
            rows = list(ex.map(one, [(lang, line) for line in lines]))
        with open(out, "w", encoding="utf-8") as f:
            f.write("\n".join(rows) + "\n")
        print(f"{lang:20} {len(rows)}")


if __name__ == "__main__":
    main()
