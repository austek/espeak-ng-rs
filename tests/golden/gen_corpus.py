#!/usr/bin/env python3
"""Build tests/golden/corpus/<voice>.txt from the C espeak-ng source tree.

    python3 -I tests/golden/gen_corpus.py --c-src DIR [--langs a,b] [--force]

DIR is a checkout of C espeak-ng (it holds dictsource/ and tests/). Each corpus
mixes words from <lang>_list/_listx, native-script names from <lang>_emoji,
match strings from <lang>_rules, inputs of the C test_phon tests, short
sentences joined from those words, and a fixed set of numbers, dates,
punctuation and abbreviations. Output is deterministic. Existing corpus files
are kept unless --force is given.
"""
import glob
import os
import random
import re
import sys
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))


def arg(name, default):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


SRC = arg("--c-src", None)
OUT = os.path.join(HERE, "corpus")

DICT_VOICES = (
    "ab af am an as az ba be bg bn bpy bs ca chr cmn crh cs cv cy da el eo et eu fi fo ga gd gn grc gu "
    "hak haw he hi hr ht hu hy ia id io is ja jbo ka kaa kk kl kn ko kok ky la lb lfn lij lt lv mi mk "
    "ml mn mr ms mt mto my nb nci ne nog om or pa pap piqd ps py qdb qu quc qya ro rup sd shn si sjn "
    "sk sl smj sq sr sv sw ta te th ti tk tn tt ug uk ur uz vi xex yue"
).split()

VARIANTS = {
    "ca-ba": "ca", "ca-nw": "ca", "ca-va": "ca",
    "cmn-latn-pinyin": "cmn", "en-029": "en", "en-gb": "en", "en-gb-scotland": "en",
    "en-gb-x-gbclan": "en", "en-gb-x-gbcwmd": "en", "en-gb-x-rp": "en", "en-shaw": "en",
    "en-us": "en", "en-us-nyc": "en", "es-419": "es", "fa-latn": "fa", "fr-be": "fr",
    "fr-ch": "fr", "hyw": "hy", "ltg": "lv", "pt-br": "pt", "ps-x-northwest": "ps",
    "ps-x-southeast": "ps", "ps-x-yusufzai": "ps", "ru-cl": "ru", "ru-lv": "ru",
    "vi-vn-x-central": "vi", "vi-vn-x-south": "vi", "yue-latn-jyutping": "yue",
}
SOURCE_ALIAS = {"nb": "no"}

FIXED = (
    "0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 30 99 100 101 1000 1234 2024 12345 1000000 "
    "3.14 0.5 1,5 1,000 -5 +7 50% $5 €10 1st 2nd 3rd 4. 1. 12:30 08:15 25/12/2024 2024-01-15 1.1.2000 "
    "a,b;c:d?e! (x) [y] {z} ... — !? 123abc abc123 A.B.C. U.S.A. 3x4 10km 5kg #1 @home x=y+z 100% ½ ² 20°"
).split() + [
    "Hello, world!", "a b c", "Dr. Smith", "etc.", "e.g. this", "No. 5", "a, b; c: d?", '"quoted" text',
    "tel. 555-1234", "It is 5 o'clock.", "Page 12 of 300.", "10.5 kg", "1 234 567", "3 + 4 = 7",
]


def is_wordish(s):
    ok = all(c.isalpha() or unicodedata.category(c).startswith("M") or c in "-'" for c in s)
    return bool(s) and ok and any(c.isalpha() for c in s)


def read_lines(path):
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8", errors="replace") as f:
        return f.read().splitlines()


def list_words(src, code):
    out = []
    for suffix in ("_list", "_listx"):
        for line in read_lines(os.path.join(src, "dictsource", code + suffix)):
            parts = line.split("//")[0].split()
            if parts and is_wordish(parts[0]):
                out.append(parts[0])
    return out


def rule_matches(src, code):
    out = []
    for line in read_lines(os.path.join(src, "dictsource", code + "_rules")):
        line = line.split("//")[0].strip()
        parts = line.split()
        if len(parts) < 2 or line[0] == ".":
            continue
        body = " ".join(parts[:-1])
        body = re.sub(r"^\S*\)\s*", "", body)
        body = re.sub(r"\s*\(\S*$", "", body).strip()
        if is_wordish(body):
            out.append(body)
    return out


def emoji_names(src, code):
    out = []
    for line in read_lines(os.path.join(src, "dictsource", code + "_emoji")):
        if line.startswith(("//", ".", "$")) or "\t" not in line:
            continue
        name = line.split("\t")[1].strip()
        if name and "[" not in name and "//" not in name:
            out.append(name)
    return out


TEST_RE = re.compile(r'^test_phon\s+(\S+)\s+"(?:[^"\\]|\\.)*"\s+"((?:[^"\\]|\\.)*)"')


def test_inputs(src):
    table = {}
    for path in sorted(glob.glob(os.path.join(src, "tests", "*.test"))):
        for line in read_lines(path):
            m = TEST_RE.match(line)
            if m:
                table.setdefault(m.group(1).lower(), []).extend(m.group(2).replace('\\"', '"').split(" "))
    return table


def sample(seq, n):
    seq = list(dict.fromkeys(seq))
    if len(seq) <= n:
        return seq
    step = len(seq) / n
    return [seq[int(i * step)] for i in range(n)]


def sentences(words, rng, plain, punct):
    if len(words) < 3:
        return []
    pick = lambda: [rng.choice(words) for _ in range(3)]
    out = [" ".join(pick()) for _ in range(plain)]
    out += ["{}, {}. {}?".format(*pick()) for _ in range(punct)]
    return out


def build(src, voice, code, tests, small):
    n_list, n_emoji, n_rules, n_tests, n_sent = (40, 15, 25, 20, 10) if small else (70, 35, 55, 40, 20)
    code = SOURCE_ALIAS.get(code, code)
    rng = random.Random(voice)
    words = sample(list_words(src, code), n_list)
    rules = sample(rule_matches(src, code), n_rules)
    given = [t for k in (voice.lower(), code) for t in tests.get(k, [])]
    pool = words + rules
    rows = words + sample(emoji_names(src, code), n_emoji) + rules + sample(given, n_tests)
    rows += sentences(pool, rng, n_sent * 3 // 5, n_sent * 2 // 5)
    rows += FIXED
    return [r for r in dict.fromkeys(x.strip() for x in rows) if r and "\t" not in r]


def main():
    if not SRC:
        sys.exit("--c-src DIR required")
    only = arg("--langs", "")
    wanted = set(only.split(",")) if only else None
    tests = test_inputs(SRC)
    jobs = [(v, v, False) for v in DICT_VOICES] + [(v, p, True) for v, p in VARIANTS.items()]
    os.makedirs(OUT, exist_ok=True)
    for voice, code, small in jobs:
        path = os.path.join(OUT, voice + ".txt")
        if (wanted and voice not in wanted) or (os.path.exists(path) and "--force" not in sys.argv):
            continue
        rows = build(SRC, voice, code, tests, small)
        with open(path, "w", encoding="utf-8") as f:
            f.write("\n".join(rows) + "\n")
        print(f"{voice:20} {len(rows)}")


if __name__ == "__main__":
    main()
