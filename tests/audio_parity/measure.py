#!/usr/bin/env python3
"""Measure audio distance between two render directories.

    measure.py --ref DIR --test DIR [--corpus corpus.tsv] --out measure.tsv [--jobs 8]
    measure.py --summary measure.tsv

DIR holds <id>.wav and ipa.tsv as written by render.py. The measurement run
writes one row per corpus input and prints the summary; --summary reprints it
from an existing file.
"""
import argparse
import csv
import math
import os
import sys
from concurrent.futures import ProcessPoolExecutor

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import metrics  # noqa: E402

METRICS = ["dur_ratio", "mcd", "lsd", "f0corr", "f0_st_mad"]
COLUMNS = ["id", "lang", "text", "ipa_match", "x_match", *METRICS]
NAN_ROW = {m: float("nan") for m in METRICS}


def read_corpus(path):
    with open(path, encoding="utf-8") as f:
        return [line.split("\t", 1) for line in f.read().splitlines() if line]


def read_ipa(directory):
    with open(os.path.join(directory, "ipa.tsv"), encoding="utf-8") as f:
        rows = (line.split("\t") for line in f.read().splitlines())
        return {r[0]: (r[1], r[2] if len(r) > 2 else "") for r in rows}


def measure_one(job):
    row_id, ref_wav, test_wav = job
    if not (os.path.exists(ref_wav) and os.path.exists(test_wav)):
        return row_id, dict(NAN_ROW)
    return row_id, metrics.compare(metrics.features(ref_wav), metrics.features(test_wav))


def run_measure(args):
    corpus = read_corpus(args.corpus)
    ref_ipa, test_ipa = read_ipa(args.ref), read_ipa(args.test)
    ids = [f"{i:04d}" for i in range(1, len(corpus) + 1)]
    jobs = [(i, os.path.join(args.ref, i + ".wav"), os.path.join(args.test, i + ".wav")) for i in ids]
    with ProcessPoolExecutor(args.jobs) as pool:
        scores = dict(pool.map(measure_one, jobs, chunksize=4))
    with open(args.out, "w", encoding="utf-8", newline="") as f:
        out = csv.writer(f, delimiter="\t", lineterminator="\n")
        out.writerow(COLUMNS)
        for row_id, (lang, text) in zip(ids, corpus):
            ipa_ok = int(ref_ipa.get(row_id, ("", ""))[0] == test_ipa.get(row_id, ("?", ""))[0])
            x_ok = int(ref_ipa.get(row_id, ("", ""))[1] == test_ipa.get(row_id, ("", "?"))[1])
            out.writerow([row_id, lang, text, ipa_ok, x_ok, *(f"{scores[row_id][m]:.4f}" for m in METRICS)])


def load_rows(path):
    with open(path, encoding="utf-8", newline="") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def stat(values, q):
    clean = [v for v in values if not math.isnan(v)]
    return float(np.percentile(clean, q)) if clean else float("nan")


def metric_values(rows, name):
    return [float(r[name]) for r in rows]


def describe(label, rows):
    if not rows:
        return None
    cells = []
    for name in METRICS:
        values = metric_values(rows, name)
        cells.append(" ".join(f"{stat(values, q):7.2f}" for q in (50, 90, 99)))
    return f"{label:<16}{len(rows):>5}  " + "   ".join(cells)


def print_summary(rows):
    header = f"{'group':<16}{'n':>5}  " + "   ".join(f"{m:^23}" for m in METRICS)
    sub = f"{'':<16}{'':>5}  " + "   ".join(f"{'p50':>7} {'p90':>7} {'p99':>7}" for _ in METRICS)
    print(header)
    print(sub)
    for lang in sorted({r["lang"] for r in rows}) + ["ALL"]:
        mine = rows if lang == "ALL" else [r for r in rows if r["lang"] == lang]
        same = [r for r in mine if r["ipa_match"] == "1"]
        diff = [r for r in mine if r["ipa_match"] != "1"]
        rate = 100.0 * len(same) / len(mine)
        print(f"-- {lang}: {len(mine)} inputs, phoneme match {rate:.1f}%")
        for label, subset in ((f"{lang} match", same), (f"{lang} mismatch", diff)):
            line = describe(label, subset)
            if line:
                print(line)


def print_worst(rows, count=10):
    ranked = sorted(rows, key=lambda r: -(float(r["mcd"]) if not math.isnan(float(r["mcd"])) else -1))
    print(f"\nworst {count} by mcd")
    for r in ranked[:count]:
        print(f"{r['id']} {r['lang']} ipa_match={r['ipa_match']} mcd={r['mcd']} lsd={r['lsd']} "
              f"dur={r['dur_ratio']} f0corr={r['f0corr']} text={r['text'][:40]!r}")


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--ref")
    p.add_argument("--test")
    p.add_argument("--corpus", default=os.path.join(HERE, "corpus.tsv"))
    p.add_argument("--out")
    p.add_argument("--jobs", type=int, default=8)
    p.add_argument("--summary")
    args = p.parse_args()
    if args.summary:
        rows = load_rows(args.summary)
    elif args.ref and args.test and args.out:
        run_measure(args)
        rows = load_rows(args.out)
    else:
        sys.exit("use --ref/--test/--out, or --summary FILE")
    print_summary(rows)
    print_worst(rows)


if __name__ == "__main__":
    main()
