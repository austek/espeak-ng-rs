#!/usr/bin/env python3
"""Deterministic sample of tests/golden/corpus for the audio-parity run.

Prints `lang<TAB>text` rows to stdout; tests/audio_parity/corpus.tsv is this
output. Seed, quotas and length limits are fixed below, so the sample is
reproducible. Half of each quota comes from lines of at least LONG characters
(longer audio gives steadier metrics), the rest from the remaining lines.
"""
import os
import random

SEED = 20261111
QUOTAS = {"ku": 110, "en": 60, "ru": 50, "de": 45, "es": 45, "fr": 45, "ar": 45}
LONG = 10
MIN_LEN = 3
MAX_LEN = 120
HERE = os.path.dirname(os.path.abspath(__file__))
CORPUS = os.path.join(HERE, "..", "golden", "corpus")

# The golden corpus is mostly single words. These rows add whole sentences and
# inputs where the port is known to differ from C in phonemes, so the
# phoneme-mismatch group of the calibration is not empty.
EXTRA = [
    ("ku", "Silav! Çawa yî? Baş im, spas."),
    ("ku", "Ez 3 sêv dixwim."),
    ("ku", "Yek - du"),
    ("ku", "5 - 3"),
    ("ku", "a @ b"),
    ("ku", "a & b"),
    ("ku", "a . b"),
    ("ku", "1,001"),
    ("ku", "(Ew hat) û çû."),
    ("en", "Hello world, this is a test of the speech system."),
    ("en", "St. Paul"),
    ("en", "Mr. X"),
    ("en", "8,848"),
    ("en", "No. 5"),
    ("en", 'He said, "Let\'s go."'),
    ("en", "The quick brown fox jumps over the lazy dog."),
    ("ru", "Привет, как дела? Всё хорошо."),
    ("ru", "XXI"),
    ("de", 'Er sagte: "Komm bitte mit."'),
    ("de", "14:30 Uhr"),
    ("es", "¿Cómo estás? Muy bien, gracias."),
    ("fr", "Bonjour, comment allez-vous aujourd'hui ?"),
    ("fr", "le 1.re"),
    ("fr", "c.-à-d."),
    ("ar", "مرحبا كيف حالك"),
]


def load(lang):
    with open(os.path.join(CORPUS, lang + ".txt"), encoding="utf-8") as f:
        lines = f.read().splitlines()
    usable = [t for t in lines if MIN_LEN <= len(t) <= MAX_LEN and "\t" not in t]
    return list(dict.fromkeys(usable))


def sample(lang, quota):
    rng = random.Random(f"{SEED}:{lang}")
    lines = load(lang)
    long_lines = [t for t in lines if len(t) >= LONG]
    picked = rng.sample(long_lines, min(len(long_lines), quota // 2))
    chosen = set(picked)
    rest = [t for t in lines if t not in chosen]
    return picked + rng.sample(rest, min(len(rest), quota - len(picked)))


def main():
    for lang, quota in QUOTAS.items():
        for text in sample(lang, quota):
            print(f"{lang}\t{text}")
    for lang, text in EXTRA:
        print(f"{lang}\t{text}")


if __name__ == "__main__":
    main()
