# Audio parity (calibration)

Measures how far the Rust port's audio is from C espeak-ng master. Not wired into CI, no ceilings yet.

Inputs: `corpus.tsv`, 420 rows (`lang<TAB>text`). `sample_corpus.py` writes it: a seeded sample (seed 20261111) of `tests/golden/corpus` (ku 110, en 60, ru 50, de 45, es 45, fr 45, ar 40) plus 25 hand-written rows (sentences, and inputs where the port is known to differ from C in phonemes).

## Run

Needs Python 3 with the packages in `requirements.txt` (use a venv outside the repo), `ffmpeg` for `degrade.py`, a C espeak-ng master build, and `cargo build --release`.

    python3 -m venv /tmp/ap-venv && /tmp/ap-venv/bin/pip install -r tests/audio_parity/requirements.txt
    PY=/tmp/ap-venv/bin/python
    $PY -I tests/audio_parity/render.py --engine c --out /tmp/ap/c --c-bin <C>/src/espeak-ng --c-data <C build dir>
    $PY -I tests/audio_parity/render.py --engine rust --out /tmp/ap/rust
    $PY -I tests/audio_parity/measure.py --ref /tmp/ap/c --test /tmp/ap/rust --out /tmp/ap/measure.tsv

`render.py` runs both engines with `-v <lang> -s 175 -p 50 -a 100 -w <wav>` and records `--ipa` and `-x` output in `ipa.tsv`. Rust uses the repo's `espeak-ng-data`; C needs the data of the same revision (the golden baseline used `f0fb11a8`). Output is 22.05 kHz mono 16-bit for both.

Self check and sensitivity check: render each engine twice and compare (identical audio must read zero), and run `degrade.py --kind tempo|pitch` on a C render and compare against the original.

## Metrics

Computed after aligning the two recordings with DTW on the mel cepstrum (10 ms hop).

| metric | meaning |
|---|---|
| `dur_ratio` | test duration / reference duration |
| `mcd` | mel-cepstral distortion in dB, coefficients 1..12 of a 40-band mel cepstrum |
| `lsd` | log-spectral distance in dB, RMS over bins, mean over the path |
| `f0corr` | Pearson correlation of voiced F0 (YIN) in semitones along the path |
| `f0_st_mad` | median absolute F0 difference in semitones along the path |
| `ipa_match`, `x_match` | `--ipa` and `-x` strings identical between the engines |

## Calibration, dengjen tip b9f477a

Median / p90 / p99 over all 420 inputs.

| comparison | dur_ratio | mcd | lsd | f0_st_mad |
|---|---|---|---|---|
| C vs C (second render) | 1.00 / 1.00 / 1.00 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Rust vs Rust (second render) | 1.00 / 1.00 / 1.00 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| C vs C, 5% faster (ffmpeg atempo) | 0.93 / 0.94 / 0.94 | 3.8 / 8.6 / 11.4 | 0.9 / 2.0 / 2.7 | 0.00 / 0.03 / 0.08 |
| C vs C, +2 semitones (resampling, formants shift too) | 0.97 / 0.98 / 0.99 | 32.7 / 40.7 / 46.9 | 5.7 / 6.8 / 7.8 | 2.00 / 2.03 / 2.09 |
| C vs Rust, phonemes identical (411 inputs) | 1.00 / 1.19 / 1.50 | 38.6 / 50.2 / 57.9 | 6.0 / 7.7 / 8.6 | 3.3 / 18.9 / 24.0 |
| C vs Rust, phonemes differ (9 inputs) | 0.97 / 1.32 / 1.39 | 46.4 / 50.9 / 51.2 | 7.9 / 8.3 / 8.4 | 2.1 / 11.9 / 12.1 |

Per language, C vs Rust, phonemes identical, median MCD: ar 43.8, de 39.1, en 37.6, es 34.9, fr 38.6, ku 36.9, ru 39.1.

Reading: both engines render byte-identical audio on repeated runs, so the metrics have a zero floor. They detect a 5% tempo change and a 2-semitone voice shift. The Rust output sits as far from C as the 2-semitone voice shift does, even where the phonemes are identical, so the phoneme ratchet does not imply audio parity.

Rust F0 is bimodal: in en, about half of the voiced frames fall at 60-120 Hz like C, the rest at 200-400 Hz (median 165 Hz against C's 87 Hz; es 172 against 95; ru 98 against 95; fr 99 against 96). In one frame at 219 Hz the spectrum is a single peak, where C has a harmonic series at 96 Hz. Cause not investigated.

## Thresholds supported by the numbers

- `mcd` below 6 dB: same (5% tempo change median 3.8); above 15 dB: different. C vs Rust today is far above.
- `lsd` below 3 dB: same (tempo change p99 2.7); above 5 dB: different.
- `f0_st_mad` below 0.5 semitone: same; 2 or more: different.
- `dur_ratio` between 0.9 and 1.1: same.

These come from one calibration run; set ceilings only after the synthesizer gap is understood.
