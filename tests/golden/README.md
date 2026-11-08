# Golden phoneme corpus

Baseline: C espeak-ng upstream commit `f0fb11a8` (master, 2026-11-07), Release build with USE_MBROLA, LIBSONIC and PCAUDIO off.

`corpus/<lang>.txt` holds one input per line for 13 languages (25757 inputs). `baseline/<lang>.tsv` holds the C output per line: text, IPA (`--ipa`), espeak phonemes (`-x`).

The Rust port is not bit-identical to C. The ratchet does not demand zero differences. It caps the number of differing rows per language at the value in `ceilings.json`, so a change can fix rows but must not break rows that match.

## Run

    cargo build --release
    python3 -I tests/golden/check.py

The script runs `target/release/espeak-ng-rs` with `ESPEAK_DATA_PATH` set to the repo's `espeak-ng-data`, counts differing rows per language and exits 1 if any language exceeds its ceiling. Options: `--binary`, `--data`, `--out DIR` (keep the captured TSVs), `--show N` (print N differing rows per language).

CI sets `ESPEAK_DATA_PATH` to the repo's bundled `espeak-ng-data`, so runs do not depend on the runner's apt package. The ceilings hold only for that data: the apt `espeak-ng-data` gives different counts (for example fa 370, ru 99). CI does not run the ignored `switch_closing_tag` tests, which expect the language-switch markers of the system data; run them locally with `ESPEAK_NG_DATA=/usr/lib/x86_64-linux-gnu/espeak-ng-data cargo test --release --test switch_closing_tag -- --ignored`.

## Lower a ceiling

When a change fixes rows, the script prints `LOWER THE CEILING` for that language. Run

    python3 -I tests/golden/check.py --write-ceilings

and commit the new `ceilings.json`. Ceilings may only go down. Raising one needs a stated reason in the commit.

## Measured on dengjen.1 (bundled data)

| lang | ceiling |
|------|---------|
| ar | 8 |
| de | 78 |
| en | 160 |
| es | 6 |
| fa | 0 |
| fr | 53 |
| it | 29 |
| ku | 32 |
| nl | 61 |
| pl | 18 |
| pt | 729 |
| ru | 2 |
| tr | 9 |
