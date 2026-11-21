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

## WAV and event ratchet

`check.py` covers only `--ipa` and `-x`. `wav_events.py` covers the audio and the synthesis events on a sample of the same corpora.

Sampling rule: for each of the 151 corpora, row `i*len//40` for `i` in 0..40, or every row when the corpus has 40 or fewer. The rule is deterministic, so a corpus edit moves the sample and needs a recapture (the check refuses to run on a stale baseline). 6040 rows in all.

`wav/<lang>.tsv` holds per row: text, md5 (first 16 hex digits) of the WAV file C `espeak-ng -w -v <lang>` writes, md5 of the event transcript, and the C sample total. The transcript is `probe.c evflat` output for options 0, flags `0x1000` (the CLI's end pause): one `ev <type> <pos> <len> <apos> <sample> <value>` line per event, then `samples=<n>`. The Rust side runs `espeak-ng-rs -w` for the WAV and the ignored test `tests/golden_events.rs` for the events, in the same format. `wav_ceilings.json` holds per language the number of rows whose WAV (`wav`) and events (`events`) differ from C. Both only go down.

Breath voices: C seeds the `breath` noise from `time(NULL)` (`speech.c`) and the C CLI has no fixed-seed option, so the WAV bytes of a voice file with a `breath` line change on every run. For those voices (`mn` among the corpora) the WAV column is the file length (`len<bytes>`) instead of the md5; events and sample totals are exact. Two captures give identical files.

Check (about 1 minute on 8 cores, 3 to 4 minutes on a CI runner):

    cargo build --release
    python3 -I tests/golden/wav_events.py

Options: `--binary`, `--data`, `--events-bin` (prebuilt `golden_events` executable; cargo builds it otherwise), `--langs a,b`, `--show N` (print N differing rows per language). It exits 1 when a language exceeds a ceiling, or has none. The logic is tested by `python3 -I tests/golden/test_wav_events.py`, which also checks that every baseline holds exactly the sampled rows.

Capture the baseline from C `f0fb11a8` (build from `git archive f0fb11a8`, not a stale checkout; static build with USE_MBROLA, LIBSONIC, LIBPCAUDIO and ASYNC off). Link the probe against the C libraries:

    gcc -O1 -I <c>/src/include -o probec capi/tests/c/probe.c <b>/src/libespeak-ng/libespeak-ng.a <b>/src/speechPlayer/libspeechPlayer.a <b>/src/ucd-tools/libucd.a -lstdc++ -lm -lpthread
    python3 -I tests/golden/wav_events.py --capture --c-bin <b>/src/espeak-ng --c-probe ./probec --c-data <b> [--langs a,b]

`<c>` is the C source tree and `<b>` its CMake build directory (`<b>/espeak-ng-data` is the data). Then set the ceilings to the true counts with `python3 -I tests/golden/wav_events.py --write-ceilings` (only for the captured languages; other entries stay).

Lower a ceiling: when the check prints `LOWER THE CEILING`, run `--write-ceilings` and commit `wav_ceilings.json`. Raising one needs a stated reason in the commit.

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
