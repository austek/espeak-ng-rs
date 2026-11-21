#!/usr/bin/env python3
"""Golden WAV and event ratchet.

Samples SAMPLE rows per language from tests/golden/corpus, synthesises each with
the Rust port and compares the WAV md5 and the synthesis-event transcript with
the C espeak-ng baseline in tests/golden/wav/<lang>.tsv. Fails if a language
has more differing rows than tests/golden/wav_ceilings.json allows.

    python3 -I tests/golden/wav_events.py [--binary PATH] [--data DIR] [--events-bin PATH]
                                          [--langs a,b] [--show N] [--write-ceilings]
    python3 -I tests/golden/wav_events.py --capture --c-bin PATH --c-probe PATH --c-data DIR
                                          [--langs a,b]

--binary is the Rust espeak-ng-rs release binary (WAV via -w). The Rust events
come from the ignored test tests/golden_events.rs; --events-bin names its
prebuilt executable, otherwise cargo builds it. --c-bin is the C espeak-ng CLI,
--c-probe the C probe (capi/tests/c/probe.c) linked against the C library, and
--c-data the directory that holds the C espeak-ng-data.
"""
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
SAMPLE = 40
FLAGS = "0x1000"
CEILINGS = os.path.join(HERE, "wav_ceilings.json")
KINDS = ("wav", "events")


def arg(name, default):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


def sample_rows(lines, n=SAMPLE):
    """Row i*len//n for i in range(n); every row when the corpus has at most n."""
    if len(lines) <= n:
        return list(lines)
    return [lines[i * len(lines) // n] for i in range(n)]


def digest(data):
    return hashlib.md5(data).hexdigest()[:16]


def corpus_sample(lang):
    with open(os.path.join(HERE, "corpus", lang + ".txt"), encoding="utf-8") as f:
        return sample_rows(f.read().splitlines())


def read_baseline(lang):
    with open(os.path.join(HERE, "wav", lang + ".tsv"), encoding="utf-8") as f:
        return [line.split("\t") for line in f.read().splitlines()]


def count_diffs(base, rows):
    """Differing (wav, events) row counts; a missing or extra row counts for both."""
    wav = ev = abs(len(base) - len(rows))
    for b, r in zip(base, rows):
        wav += b[1] != r[1]
        ev += b[2] != r[2]
    return {"wav": wav, "events": ev}


def is_stale(base, rows):
    """True when the baseline rows are not the sampled rows."""
    return len(base) != len(rows) or any(b[0] != r[0] for b, r in zip(base, rows))


def verdict(measured, ceilings):
    """(failed, report lines) for measured {lang: {kind: n}} against ceilings."""
    failed, lines = False, []
    for lang in sorted(measured):
        ceil = ceilings.get(lang)
        parts, status = [], "OK"
        for kind in KINDS:
            n = measured[lang][kind]
            c = None if ceil is None else ceil.get(kind)
            parts.append(f"{kind} {n} ceiling {c}")
            if c is None or n > c:
                status, failed = ("NO CEILING" if c is None else "FAIL"), True
            elif n < c and status == "OK":
                status = "LOWER THE CEILING"
        lines.append(f"{lang:16} {'  '.join(parts)}  {status}")
    return failed, lines


def has_breath(data, lang):
    """True when the voice file of lang has a breath line (C seeds its noise from time(NULL))."""
    for root in (data, os.path.join(data, "espeak-ng-data")):
        base = os.path.join(root, "lang")
        for d in (os.listdir(base) if os.path.isdir(base) else []):
            path = os.path.join(base, d, lang)
            if os.path.isfile(path):
                with open(path, encoding="utf-8", errors="replace") as f:
                    return any(line.startswith("breath") for line in f)
    return False


def wav_column(wav, breath):
    """md5 of the WAV bytes; byte length only for breath voices, whose noise is random in C."""
    return f"len{len(wav)}" if breath else digest(wav)


def run_wav(binary, data, lang, text, tmp):
    path = os.path.join(tmp, lang + ".wav")
    env = dict(os.environ, ESPEAK_DATA_PATH=data)
    try:
        r = subprocess.run([binary, "-w", path, "-v", lang, "--", text], capture_output=True, env=env, timeout=60)
    except subprocess.TimeoutExpired:
        return "TIMEOUT"
    if r.returncode or not os.path.exists(path):
        return f"ERR{r.returncode}"
    with open(path, "rb") as f:
        out = wav_column(f.read(), has_breath(data, lang))
    os.remove(path)
    return out


def escape_probe(text):
    return text.replace("\\", "\\x5c")


def c_events(probe, data, lang, text):
    env = dict(os.environ, ESPEAK_DATA_PATH=data)
    try:
        r = subprocess.run([probe, "evflat", "0", lang, FLAGS, escape_probe(text)],
                           capture_output=True, env=env, timeout=60)
    except subprocess.TimeoutExpired:
        return "TIMEOUT", ""
    if r.returncode:
        return f"ERR{r.returncode}", ""
    return digest(r.stdout), r.stdout.splitlines()[-1].decode("utf-8", "replace") if r.stdout else ""


def rust_events(events_bin, data, lang, texts, tmp):
    """Per-row transcript digests from the ignored Rust test; missing rows are ERR."""
    src = os.path.join(tmp, lang + ".rows")
    with open(src, "w", encoding="utf-8") as f:
        f.write("\n".join(texts) + "\n")
    env = dict(os.environ, GOLDEN_EV_FILE=src, GOLDEN_EV_LANG=lang, GOLDEN_EV_DATA=data)
    try:
        r = subprocess.run([events_bin, "--ignored", "--nocapture", "--exact", "dump"],
                           capture_output=True, env=env, timeout=600)
        out = r.stdout
    except subprocess.TimeoutExpired as e:
        out = e.stdout or b""
    return parse_events(out, len(texts))


def parse_events(out, n):
    """Digest per row from the dumper output; a row without a transcript, or with ERR, is ERR."""
    chunks, cur = {}, None
    for raw in out.split(b"\n"):
        if raw.startswith(b"=== "):
            cur = int(raw[4:])
            chunks[cur] = []
        elif cur is not None and raw.startswith((b"ev ", b"samples=", b"ERR")):
            chunks[cur].append(raw)
    return [digest(b"\n".join(chunks[i]) + b"\n") if i in chunks and chunks[i] != [b"ERR"] else "ERR"
            for i in range(n)]


def find_events_bin():
    r = subprocess.run(["cargo", "test", "--release", "--test", "golden_events", "--no-run",
                        "--message-format=json"], capture_output=True, cwd=ROOT)
    if r.returncode:
        sys.exit(r.stderr.decode("utf-8", "replace"))
    for line in reversed(r.stdout.splitlines()):
        try:
            exe = json.loads(line).get("executable")
        except ValueError:
            continue
        if exe and "golden_events" in os.path.basename(exe):
            return exe
    sys.exit("golden_events executable not found")


def measure_rust(lang, binary, data, events_bin, tmp):
    texts = corpus_sample(lang)
    evs = rust_events(events_bin, data, lang, texts, tmp)
    return [[t, run_wav(binary, data, lang, t, tmp), e] for t, e in zip(texts, evs)]


def capture_c(lang, c_bin, c_probe, c_data, tmp):
    rows = []
    for t in corpus_sample(lang):
        evh, samples = c_events(c_probe, c_data, lang, t)
        rows.append([t, run_wav(c_bin, c_data, lang, t, tmp), evh, samples])
    return rows


def write_rows(lang, rows):
    os.makedirs(os.path.join(HERE, "wav"), exist_ok=True)
    with open(os.path.join(HERE, "wav", lang + ".tsv"), "w", encoding="utf-8") as f:
        f.write("\n".join("\t".join(r) for r in rows) + "\n")


def langs():
    found = sorted(fn[:-4] for fn in os.listdir(os.path.join(HERE, "corpus")) if fn.endswith(".txt"))
    only = arg("--langs", "")
    return [x for x in found if x in only.split(",")] if only else found


def capture_main():
    c_bin, c_probe, c_data = arg("--c-bin", None), arg("--c-probe", None), arg("--c-data", None)
    if not (c_bin and c_probe and c_data):
        sys.exit("--c-bin, --c-probe and --c-data required")
    with tempfile.TemporaryDirectory() as tmp, ThreadPoolExecutor(8) as ex:
        jobs = {lang: ex.submit(capture_c, lang, c_bin, c_probe, c_data, tmp) for lang in langs()}
        for lang, job in jobs.items():
            write_rows(lang, job.result())
            print(f"{lang:16} captured")
    return 0


def check_main():
    binary = arg("--binary", os.path.join(ROOT, "target", "release", "espeak-ng-rs"))
    data = arg("--data", os.path.join(ROOT, "espeak-ng-data"))
    show = int(arg("--show", "0"))
    events_bin = arg("--events-bin", None) or find_events_bin()
    selected = langs()
    measured = {}
    with tempfile.TemporaryDirectory() as tmp, ThreadPoolExecutor(8) as ex:
        jobs = {lang: ex.submit(measure_rust, lang, binary, data, events_bin, tmp) for lang in selected}
        for lang, job in jobs.items():
            rows, base = job.result(), read_baseline(lang)
            if is_stale(base, rows):
                sys.exit(f"{lang}: baseline rows do not match the sampling rule; recapture")
            measured[lang] = count_diffs(base, rows)
            for b, r in [(b, r) for b, r in zip(base, rows) if b[1:3] != r[1:3]][:show]:
                print(f"   {lang} {b[0]!r}: wav {b[1]} -> {r[1]}  events {b[2]} -> {r[2]}")
    if "--write-ceilings" in sys.argv:
        merged = json.load(open(CEILINGS, encoding="utf-8")) if os.path.exists(CEILINGS) else {}
        merged.update(measured)
        with open(CEILINGS, "w", encoding="utf-8") as f:
            json.dump(merged, f, indent=2, sort_keys=True)
            f.write("\n")
        print("ceilings written")
        return 0
    with open(CEILINGS, encoding="utf-8") as f:
        failed, lines = verdict(measured, json.load(f))
    print("\n".join(lines))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(capture_main() if "--capture" in sys.argv else check_main())
