## Known deviations

Hand-maintained; `gaps.py` copies this file into `GAPS.md`. Outputs are `-q --ipa` unless noted, C is espeak-ng f0fb11a8.

### Deliberate, or C bugs not ported

| input | C | port | reason |
|---|---|---|---|
| `-v en` `ａｂｃ` (full-width letters) | `lˌɛtəˌɛfˌɛffˈɔːwˌɒn...` (letter name, then the hex code point `FF41`) | `ˌeɪbˌiːsˈiː` | The port folds full-width forms to ASCII, so the text is read as `abc`. |
| `-v en` `１２３` (full-width digits) | empty output | `wˈɒnhˈʌndɹɪdən twˈɛnti θɹˈiː` | C drops full-width digits; the port reads them as `123`. |
| `-v en` `a،b` (Arabic comma) | `ɐ ˈaɹəbɪk(ar)fˈaːs̪-ˌila(en) bˈiː` | `ɐ ʑʍ bˈiː` | C switches to the Arabic voice and prints garbage bytes for the unnamed mark. The same happens in tr (`a،b`: C `ˈa (en)ˈaɹəbɪk(tr)fˈaːøˌila bˈe`, port `ˈa ʑaʊ bˈe`). |
| `-v hi` `1,234.5` | `ˈeːk hˈʌɟaːɾ dˈoːsˈɔː cˈɔ̃tis` | `ˈeːk hˈʌɟaːɾ dˈoːsˈɔː cˈɔ̃tis dˌəsəmlˈoː pˈãc` | C drops the fraction in lakh-grouping languages (bn, mr, gu, ta, te, kn, ml, pa show the same); the port reads it. Candidate to match C if drop-in parity outranks correctness. |
| `espeak-ng -v f2 hello` (variant name as the voice) | segfault, exit 139 (`Unknown phoneme table: ''`) | `voice not found: variant`, exit 1 | C dereferences a missing language; the port reports an error. `-v en+f2` works in both. |
| `-v cmn` `哪儿` | `(en)na5θɹˈiː5rfˈaɪ5v(cmn) ˈərɜ` (spells an English `three five` from the tone digits) | `nˈɑ2 ˈərɜ` | C leaks the tone digits into an English reading; the port does not. |
| `-w out.wav -v en+whisper hello` (also `en+f2`) | the breath noise comes from a clock-seeded generator, so two runs differ | identical on every run | The port seeds the generator with a fixed value. Output equals C only with `-D` (deterministic mode); plain `en` is identical without it. |

### Empirical

| area | note |
|---|---|
| `hyw` stress and letter groups | C `tr_languages.c` has only an `hy` case. The port gives `hyw` the `hy` stress rule and letter table (`src/dictionary/stress.rs`, `src/dictionary/file.rs`) because the output then matches C on the `hyw` corpus; no C source line names `hyw`. |

### Open, not deliberate

| input | C | port |
|---|---|---|
| `-v bs` `12:30` (same in sr, hr) | `dvˈanaɪzd dvˈotætʃkæ tɾˈidɛsɛt` | `dvˈanaɪst tɾˈidɛsɛt` (drops the colon word, no voicing of `st`) |
| `-v en` `a . b` | `ˌeɪbˈiː` | `ɐ bˈiː` |
| `-v en` `a?b` | `ˈɐb` | `ˈab` |
| `-v en` `¬¬¬¬` (also `→→`, `■■`; most voices) | one word read by the rules, `n,0tn,0tn,0tn'0t`, event `1 1 4` | the port joins the run only for a fixed block of symbols (`is_non_ascii_symbol`, not C's `IsAlpha` test) and reads each member by its list name, `n,0t s'aInn,0t s'aIn…`; fr matches. `-v de` `¬¬` is silent in C. `-v ky` `×÷` matches C (`kOb'Ojty:`) only because U+D7 and U+F7 stay out of the block; joined, the port would read nothing. ★ ✓ ♪ ‰ † are not joined at all. |
| `-v en` `a ! ! !`; `x: : :`; `( ! )`; `a_b` in 12 voices | | end events of marks after a word, the clause split of `: : :` and the word split at `_` differ; not caused by the symbol or `_` changes of #22 |
| `-v en` `--punct=_` `a _ b _ _` | `a# 'Vnd3sk,o@ b'i:;'Vnd3sk,o@r'Vnd3sk,o@` | `…b'i:; 'Vnd3sk,o@r- 'Vnd3sk,o@` (same as before #22) |
| `-v uk` `a … b` (same with `a... b`; mk, hu likewise) | `(en)a#(uk)tR'e kR'apkI` then `(en)b'i:(uk)` on a second line: `ReadClause` calls `LookupCharName(only)` for a character with the Ellipsis property and writes the bare `…` list entry into the clause as phoneme text, then ends the clause without intonation | `…` is silent; the port ends the clause at the mark and never reads the list entry. Other Ellipsis characters (U+0EAF, U+1801, U+22EE-22F1, U+FE19) have no bare list entry in these voices; where the emoji lists name them, C `f0fb11a8` aborts with `stack smashing detected` (`cn_buf[74]`, `phonemes[60]` read uninitialised when the lookup fails), so no output exists to match. |
| `--pho hello` without `-w`, non-MBROLA voice (#21) | prints nothing, exit 0 in the audio-less reference build (a build with an audio device plays the speech) | prints the `.pho` script; `tests/mbrola_cli.rs` pins this | Kept: the script is the documented port extension. |
| `-X` (#21) | rule trace lines differ from C in content (blank line after each word's rules, `(_` vs `(_N` rule text); tracked in #12 | | The per-clause order of trace and phoneme lines matches C (`tests/cli_residuals.rs`). |
| `--ipa=1`/`--sep=_` de, fr, cmn; `ko`/`ja` WAV; `fr+f2`/`en+f3` WAV; 998-character unpunctuated clause (#21) | | | Not investigated in this change; they are synthesis and phoneme-separator differences outside the CLI layer. Still open in #21. |
