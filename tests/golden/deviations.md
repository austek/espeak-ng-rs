## Known deviations

Hand-maintained; `gaps.py` copies this file into `GAPS.md`. Outputs are `-q --ipa` unless noted, C is espeak-ng f0fb11a8.

### Deliberate, or C bugs not ported

| input | C | port | reason |
|---|---|---|---|
| `-v en` `１０:３０` (full-width), `١٠:٣٠` (Arabic-Indic), `१०:३०` (Devanagari); also `10:३०`, `１２:30` (#20) | `k'oUl@n` (full-width), empty (Arabic-Indic), `_!k'oUl@n_!` (Devanagari), `10:३०` is `t'En k'oUl@n_!`; de/fr/es/... give `_!_!` for Devanagari digits | the port maps native and full-width digits to ASCII and reads the time (`t'En T'3:ti`); ar, fa and hi match because their rules name the digits | Same cause as the full-width row below: C reads a non-ASCII digit as a letter of its script (a pause or silence in a voice without rules for it), and the port deliberately reads digits in every script. Matching C would silence the digits of every language that has no native-digit rules. |
| `-v de` `5., b`, `5.; b`, `5.: b`, `5.… b` (also nl) (#20) | one clause, ordinal `f'ynft@ b'e:` | ordinal read, but the clause ends at the comma or mark (`f'ynft@` / `b'e:` on two lines) | C keeps a clause mark after the ordinal dot inside the clause (`CHAR_COMMA_BREAK` flag); the port's clause splitter ends the clause at the mark before the ordinal check runs. |
| `-v no` events of `5. b`, `5.. b`, `00. b`; `-v ja` events of `02:3 b` (#20) | word timing 304 ms / sample 6716 (no), word position 6 (ja) | 341 ms / 7530, position 5 | The word events differ from C for these voices without the #20 inputs as well (`no 5. b`, `ja 10:30 b`); `-x`, `--ipa` and WAV match. |
| `-v de` `12 345.  b`, `99 999.  maj` (also nl, hu, nb; two blanks after the dot of a space-grouped number) (#20) | de `tsv'Wlf t'aUz@nt_! dr'aIh'Und3t f'ynf _\|Untf'IrtsICst@ b'e:` (ordinal) | cardinal without the ordinal ending (base read the same) | C reads the last group as an ordinal with two blanks as with one; the port reads it as a cardinal then. fi, et and sl read a cardinal in C too. |
| `-v en` `02:3,` U+00A0 `b`, `00:0,` U+00A0 `B` (plain text; the SSML text matches), `02:3.` U+00A0 `b` (#20) | one clause, the zero kept; `d0t` for the dot | the comma ends the clause (two lines); `d'0t` for the dot | The port's clause splitter ends the clause at a comma before a no-break space, and stresses the spoken dot; both are as before #20. |
| `-v de` `5.` U+00A0 `b` (de, nl, nb, da, lv, lt; hu matches, #20) | de `f'ynf pUNkt b'e:`, nl `v'EIf b'e:`, nb `f'Em pu-Nkt,u-:m b'e:`, da `f'?Em pONt,Om b'e`, lv `p'ietsi punkts b'e:_:`, lt `p;enk'I t'aSkas b;'ee` | de `f'ynf b'e:`, nl `v'EIf p'8nt b'e:`, nb `f'Em p'u-Nktu-:m b'e:`, da `f'?Em b'e`, lv `p'ietsi b'e:_:`, lt `p;enk'I _:t'aSkas b;'ee` | C's `iswspace` is false for the no-break space, so the dot stays glued to the number and is read as a symbol (not in nl). The port no longer reads an ordinal here, as C does, but it speaks the glued dot differently; before #20 it read an ordinal. |
| `-v de` `1 005. b`, `12 345. b` (also nl, hu, nb) (#20) | `_'aIn t'aUz@nt_! f'ynft@ b'e:`, `tsv'Wlf t'aUz@nt_! dr'aIh'Und3t f'ynf _\|Untf'IrtsICst@ b'e:` | `_'aIns f'ynft@ b'e:`, `tsv'Wlf dr'aIh'Und3t f'ynf _\|Untf'IrtsICst@ b'e:` | C reads a space-grouped number with the ordinal dot as the thousands plus the ordinal of the last group. The port's space grouping merges cardinal groups only, so the leading groups are lost. The leading-zero rule of the ordinal dot has C's thousands-group exception now; the grouping stays. |
| `-v en` `02:3-b`, `02:3 _-b` (also de, fr, ru) (#20) | `t'u: k'oUl@n Tr'i:b'i:`: C joins the word after the hyphen to the minute | `t'u: k'oUl@n Tr'i: b'i:` (`02:3-b` now without the zero; `02:3 _-b` keeps it) | C joins the word and drops the zero; the port keeps the space after the minute and does not see the padding behind `_-`. |
| `-v en` `1:02:3 b`, `02:3' b`; `-v de` `1  005. b` (#20) | `w'0n t'u: k'oUl@n Tr'i: b'i:`; `t'u: k'oUl@n Tr'i:_:_: b'i:`; `_'aIns n'Uln'Ul f'ynf b'e:` | `w'0n k'oUl@n t'u: k'oUl@n Tr'i: b'i:` (base kept the zero); `z'i@roU t'u: k'oUl@n Tr'i:_:_: b'i:`; `_'aIn t'aUz@nt_! f'ynf b'e:` | A zero-led minute after another colon, an apostrophe glued behind the minute and a space-grouped number with two blanks are not read as C reads them. |
| `-v fr` `02:3.5` (#20) | `d'Y trwa _\|pwE~ s'E~k` | same phonemes | `-x` and `--ipa` match; the WAV differs in a pause length. |
| `-v en` `02:30-b`, `02:30x5`, `02:30a b`, `02:30..5` (#20) | `t'u: T'3:tib'i:`, `t'u: T'3:ti; 'Eks f'aIv`, ... | `t'u: T'3:ti b'i:`, `t'u: T'3:ti baI f'aIv`, ... | A hyphen, letter or double dot glued behind the minute changes C's word boundaries; the port tokenises these differently. |
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
