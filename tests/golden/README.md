# Golden phoneme corpus

Baseline: C espeak-ng upstream commit `f0fb11a8` (master, 2026-11-07), Release build with USE_MBROLA, LIBSONIC and PCAUDIO off.

`corpus/<lang>.txt` holds one input per line for every voice of the C build (151 corpora, 55863 inputs). The original 13 languages have 100 to 6200 inputs each; the others have 135 to 290, built by `gen_corpus.py` from the C `dictsource` (word lists, rule match strings, emoji names, `test_phon` inputs) plus a fixed set of numbers, dates, punctuation and abbreviations. `capture.py` records the C output; `gaps.py` writes `GAPS.md` (differing rows per language with samples). The full check takes about 3 minutes on 8 cores. `baseline/<lang>.tsv` holds the C output per line: text, IPA (`--ipa`), espeak phonemes (`-x`).

`gen_corpus.py` appends the per-voice `EXTRA` rows (ru, uk and mk words with long initial consonant runs) to a new corpus; the 13 original corpora, ru among them, are hand-maintained, so add an `EXTRA` row to the corpus file as well and run `capture.py --langs <lang> --force`. `deviations.md` holds the known deliberate deviations and is copied into `GAPS.md`.

The Rust port is not bit-identical to C. The ratchet does not demand zero differences. It caps the number of differing rows per language at the value in `ceilings.json`, so a change can fix rows but must not break rows that match.

## Run

    cargo build --release
    python3 -I tests/golden/check.py

The script runs `target/release/espeak-ng-rs` with `ESPEAK_DATA_PATH` set to the repo's `espeak-ng-data`, counts differing rows per language and exits 1 if any language exceeds its ceiling. Options: `--binary`, `--data`, `--out DIR` (keep the captured TSVs), `--show N` (print N differing rows per language).

CI sets `ESPEAK_DATA_PATH` to the repo's bundled `espeak-ng-data`, so runs do not depend on the runner's apt package. The ceilings hold only for that data: the apt `espeak-ng-data` gives different counts (for example fa 370, ru 99). The workflow also runs the formerly ignored `switch_closing_tag` tests against the bundled data.

## Lower a ceiling

When a change fixes rows, the script prints `LOWER THE CEILING` for that language. Run

    python3 -I tests/golden/check.py --write-ceilings

and commit the new `ceilings.json`. Ceilings may only go down. Raising one needs a stated reason in the commit.

## Measured on dengjen.3 (refreshed bundled data)

| lang | dengjen.2 | ceiling |
|------|-----------|---------|
| ar | 8 | 7 |
| de | 78 | 78 |
| en | 160 | 159 |
| es | 6 | 6 |
| fa | 0 | 0 |
| fr | 53 | 53 |
| it | 29 | 25 |
| ku | 32 | 3 |
| nl | 61 | 60 |
| pl | 18 | 16 |
| pt | 729 | 39 |
| ru | 2 | 2 |
| tr | 9 | 9 |
