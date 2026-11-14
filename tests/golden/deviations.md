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
