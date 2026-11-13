//! IPA rendering straight from the clause's phoneme-code list.
//!
//! This is a port of C's `GetTranslatedPhonemeString()` (dictionary.c) together
//! with the `use_ipa` half of `WritePhMnemonic()`.  Both walk the *finished*
//! phoneme list — the same list `-x` prints — so `--ipa` and `-x` can never
//! disagree about which phonemes a clause contains, only about how each one is
//! spelled.
//!
//! The important part is where the spelling comes from.  C does not have a
//! table of IPA names: it **runs each phoneme's bytecode program** with that
//! phoneme's real neighbours and takes whatever `ipa <string>` the executed
//! path reached (`phdata->ipa_string`).  The name is therefore contextual —
//! Italian `*` is `r` between vowels and `ɾ` elsewhere, and Spanish `i;` is
//! plain `i` rather than the `iʲ` its mnemonic would suggest.  Only when the
//! program reaches no `ipa` instruction does C fall back to mapping the
//! mnemonic's characters through `ipa1[]`.

use std::path::Path;

use crate::phoneme::{
    PhonemeData, PHON_END_WORD, PHON_STRESS_2, PHON_LENGTHEN, PHON_SYLLABIC, PHON_STRESS_3, PHON_STRESS_D, PHON_STRESS_P,
    PHON_STRESS_P2, PHON_STRESS_PREV, PHON_STRESS_TONIC, PHON_STRESS_U,
};
use crate::synthesize::bytecode::{self, interpret_phoneme, Neighbours};

use super::ipa_table::{encode_utf8, ipa1_char};
use super::{CodeMarker, PhonemeCode};

/// Private-use character that stands for the empty word between two spaces
/// until the output is finished.
const EMPTY_WORD_MARK: char = '\u{e000}';
/// Stands for the space C writes before a clause's first spoken word when a
/// silent pause phoneme came first; it must survive the line trimming.
const LEADING_SPACE_MARK: char = '\u{e001}';

/// C `WritePhMnemonic(..., use_ipa = 1)` for one phoneme of the list.
///
/// `nb` supplies the neighbours the phoneme's program conditions on; pass
/// [`Neighbours::default`] for a phoneme with no context (C's
/// `InterpretPhoneme2`, which surrounds it with pauses).
pub fn write_ph_mnemonic_ipa(phdata: &PhonemeData, code: u8, nb: &Neighbours) -> String {
    // phonEND_WORD is a list marker, not a sound.
    if code == PHON_END_WORD {
        return String::new();
    }
    let Some(ph) = phdata.get(code) else {
        return String::new();
    };

    program_ipa_name(phdata, code, nb)
        .unwrap_or_else(|| mnemonic_to_ipa_c(ph.mnemonic, ph.typ == 2 /* phVOWEL */))
}

/// The IPA name a phoneme's own program declares; `Some("")` is C's "this
/// phoneme has no IPA name" marker, `None` means the program names nothing.
fn program_ipa_name(phdata: &PhonemeData, code: u8, nb: &Neighbours) -> Option<String> {
    let ph = phdata.get(code)?;
    if ph.program == 0 {
        return None;
    }
    let fx = interpret_phoneme(ph.program, &phdata.phonindex, nb, |c| phdata.get(c).cloned());
    let s = fx.ipa_string.as_deref()?;
    let b = s.as_bytes();
    // A leading space is C's "this phoneme has no IPA name" marker.
    if b.first() == Some(&0x20) {
        return Some(String::new());
    }
    // A leading byte below 0x20 is a flags byte, not text.
    let s = match b.first() {
        Some(&c) if c < 0x20 => &s[1..],
        _ => s,
    };
    (!s.is_empty()).then(|| s.to_string())
}

/// The mnemonic fallback of `WritePhMnemonic`, character for character.
///
/// Unlike [`super::ipa_table::mnemonic_to_ipa`] this keeps a `-` in a vowel
/// mnemonic, because C does: French's reducible schwa `@-` prints `ə-`.
fn mnemonic_to_ipa_c(mnemonic: u32, is_vowel: bool) -> String {
    let mut out = Vec::new();
    let mut first = true;
    let mut mnem = mnemonic;
    while mnem != 0 {
        let c = (mnem & 0xff) as u8;
        mnem >>= 8;
        if c == 0 || c == b'/' {
            break; // end, or the phoneme-variant indicator
        }
        if first && c == b'_' {
            break; // pause phonemes are not shown
        }
        if c == b'#' && is_vowel {
            break; // `#` is subscript-h, and only for consonants
        }
        if !first && c.is_ascii_digit() {
            continue; // digits after the first character are variant numbers
        }
        encode_utf8(ipa1_char(c), &mut out);
        first = false;
    }
    String::from_utf8(out).unwrap_or_default()
}

/// Render a whole clause's code list as IPA — C's `GetTranslatedPhonemeString`
/// with `espeakPHONEMES_IPA` set.
///
/// The structure deliberately mirrors the CLI's `-x` writer: the same code list,
/// the same word/clause boundaries, the same `(lang)` switch markers.  Only the
/// per-phoneme spelling and the stress characters differ.
///
/// `preserve_punctuation` is the port's phonemizer mode: the punctuation that
/// upstream simply drops is echoed into the output, and a clause's terminator is
/// printed before its line break.
pub fn codes_to_ipa(
    codes: &[PhonemeCode],
    phdata: &PhonemeData,
    data_dir: &Path,
    lang: &str,
    preserve_punctuation: bool,
) -> String {
    codes_to_ipa_tied(codes, phdata, data_dir, lang, preserve_punctuation, None)
}

/// [`codes_to_ipa`] with the `--tie` character: it goes *inside* each phoneme's
/// IPA name, before every character after the first that is a letter, so `tʃ`
/// prints as `t͡ʃ`.  Only the writer knows where one phoneme's name ends, which
/// is why this cannot be done to the finished string.
pub fn codes_to_ipa_tied(
    codes: &[PhonemeCode],
    phdata: &PhonemeData,
    data_dir: &Path,
    lang: &str,
    preserve_punctuation: bool,
    tie: Option<char>,
) -> String {
    codes_to_ipa_opts(codes, phdata, data_dir, lang, preserve_punctuation, tie, None)
}

/// [`codes_to_ipa`] with both of upstream's phoneme-name options: `tie` goes
/// *inside* a multi-letter name, `sep` between phonemes.  Only one is ever set —
/// `--sep` and `--tie` share a slot in the CLI, as they do upstream.
///
/// The separator is placed as C places it: before every phoneme but the first
/// of a clause, never directly after a word break (unless it *is* a space, which
/// is why `--sep` doubles the space between words), and not before a name that
/// starts with a modifier letter.  A stress mark belongs to the phoneme that
/// follows it, so the separator goes before the stress: `tʃ.ˈiː.z`.
#[allow(clippy::too_many_arguments)]
pub fn codes_to_ipa_opts(
    codes: &[PhonemeCode],
    phdata: &PhonemeData,
    data_dir: &Path,
    lang: &str,
    preserve_punctuation: bool,
    tie: Option<char>,
    sep: Option<char>,
) -> String {
    // Indices of the codes that are actual sounds, for neighbour lookups.
    let is_sound =
        |c: &PhonemeCode| {
            !c.is_boundary && c.code > 8 && c.code != PHON_END_WORD && c.code != PHON_SYLLABIC
        };
    let real: Vec<usize> = codes
        .iter()
        .enumerate()
        .filter(|(_, c)| is_sound(c))
        .map(|(i, _)| i)
        .collect();
    // Position within `real` for each code index, so the loop below can find a
    // phoneme's neighbours without rescanning.
    let mut rank = vec![usize::MAX; codes.len()];
    for (n, &i) in real.iter().enumerate() {
        rank[i] = n;
    }
    let starts = super::word_starts(codes, &real);

    let mut out = String::new();
    // The table the codes that follow belong to; `None` = the clause's own.
    let mut switched: Option<String> = None;
    let mut tables = super::SwitchTables::default();
    let mut straight_quote_open = false;
    // `--sep` bookkeeping: whether a phoneme has been written in this clause,
    // whether a word break has just happened (no separator straight after one,
    // unless the separator is itself a space), and the stress mark waiting for
    // the phoneme it belongs to.
    let mut wrote_phoneme = false;
    let mut after_word_break = true;
    let mut pending_stress = String::new();
    let mut last_sound: Option<(u8, Neighbours)> = None;
    let mut pause_only_word = false;
    let mut leading_pause = false;

    let voice_phdata = phdata;
    for (i, pc) in codes.iter().enumerate() {
        if let Some(CodeMarker::Punctuation(c)) = &pc.marker {
            if preserve_punctuation {
                // An opening bracket or quote binds to the word that follows.
                let opening = match c {
                    '(' | '[' | '{' | '\u{201c}' | '\u{2018}' => true,
                    '"' => !straight_quote_open,
                    _ => false,
                };
                if *c == '"' {
                    straight_quote_open = !straight_quote_open;
                }
                if opening && !out.is_empty() && !out.ends_with([' ', '\n']) {
                    out.push(' ');
                }
                out.push(*c);
                if opening {
                    // Swallow the separator the tokenizer emits after it.
                    continue;
                }
            }
            continue;
        }
        if let Some(CodeMarker::LangSwitch(target)) = &pc.marker {
            // A *closing* marker binds to the word it follows, so the word-space
            // moves after it — as in `-x`.
            // Put back exactly as many spaces as were taken, so a space
            // separator keeps the doubling the word break already wrote.
            let mut popped_spaces = 0usize;
            while target == lang && out.ends_with(' ') {
                out.pop();
                popped_spaces += 1;
            }
            let trailing_space = popped_spaces > 0;
            // C writes the marker through `WritePhMnemonic` like a phoneme name,
            // so `--sep` separates it and `--tie` goes inside it: `(en).ð.ə.(de)`
            // and `(-e-n)`.
            let marker = format!("({})", super::switch_marker_name(phdata, target, lang));
            if let Some(sep) = sep {
                // A closing marker took the word space back, so the phoneme
                // before it is still the one to separate from.
                if wrote_phoneme && (!after_word_break || trailing_space) {
                    out.push(sep);
                }
            }
            out.push_str(&match tie {
                Some(t) => tie_phoneme_name(&marker, t),
                None => marker,
            });
            wrote_phoneme = true;
            if trailing_space {
                // Still at a word break: the next word starts a run of its own.
                for _ in 0..popped_spaces {
                    out.push(' ');
                }
            } else {
                after_word_break = false;
            }
            switched = (target != lang).then(|| target.to_string());
            continue;
        }
        let phdata: &PhonemeData = switched
            .as_deref()
            .and_then(|t| tables.get(data_dir, t))
            .unwrap_or(phdata);
        let segment = phdata;
        let phdata = super::table_for_code(phdata, voice_phdata, pc.code);

        if pc.is_boundary || pc.code == PHON_END_WORD {
            out.push_str(&std::mem::take(&mut pending_stress));
            after_word_break = true;
            if pc.is_boundary && pc.code == 0 && pc.clause_char.is_some() {
                if preserve_punctuation {
                    if let Some(c) = pc.clause_char.filter(|c| ".,!?;:".contains(*c)) {
                        while out.ends_with(' ') {
                            out.pop();
                        }
                        out.push(c);
                    }
                }
                out.push('\n');
                wrote_phoneme = false; // a new clause starts the run again
                leading_pause = false;
            } else {
                // A word that is only pauses still takes its space in C, so
                // mark it to survive the collapsing of doubled spaces below.
                if pause_only_word && out.ends_with(' ') {
                    out.push(EMPTY_WORD_MARK);
                }
                pause_only_word = false;
                let line_empty = out.is_empty() || out.ends_with('\n');
                if line_empty && leading_pause && sep.is_none() {
                    out.push(LEADING_SPACE_MARK);
                } else {
                    out.push(' ');
                }
                leading_pause = false;
                // A *space* separator is emitted at a word start too, which is
                // why `--sep` doubles the space between words — but only once,
                // even where two boundaries meet (a switched word brings its
                // own).
                if sep == Some(' ') && !out.ends_with("  ") {
                    out.push(' ');
                }
            }
            continue;
        }

        match pc.code {
            PHON_STRESS_P | PHON_STRESS_P2 | PHON_STRESS_TONIC => pending_stress.push('\u{2c8}'),
            PHON_STRESS_2 | PHON_STRESS_3 => pending_stress.push('\u{2cc}'),
            PHON_STRESS_U | PHON_STRESS_PREV | PHON_STRESS_D => {}
            0 => {}
            code => {
                // A stress *marker* phoneme (type 1 with no program) encodes its
                // level in `std_length`, exactly as the `-x` writer reads it.
                if let Some(ph) = phdata.get(code) {
                    if ph.typ == 1 && ph.program == 0 {
                        match ph.std_length {
                            4 => pending_stress.push('\u{2c8}'),
                            2 | 3 => pending_stress.push('\u{2cc}'),
                            _ => {}
                        }
                        continue;
                    }
                }
                let nb = neighbours_at(codes, &real, &starts, rank[i], phdata);
                // C marks the first phoneme of each word as a word start and
                // writes a space before every one but the clause's first.  A
                // silent pause at the clause start is that first phoneme, so a
                // word break after it (a Dutch number word) still gets its space.
                if (out.is_empty() || out.ends_with('\n'))
                    && phdata.get(code).is_some_and(|p| p.typ == crate::phoneme::PH_PAUSE)
                {
                    leading_pause = true;
                }
                pause_only_word |=out.ends_with(' ')
                    && phdata.get(code).is_some_and(|p| p.typ == crate::phoneme::PH_PAUSE);
                let name = match &last_sound {
                    // C writes the lengthener and the syllabic marker through the
                    // phoneme's own entry, so a declared IPA name is printed again.
                    Some((prev, prev_nb))
                        if (code == PHON_LENGTHEN
                            || (code == PHON_SYLLABIC
                                && phdata.get(*prev).is_some_and(|p| p.typ != 2))) =>
                    {
                        program_ipa_name(phdata, *prev, prev_nb)
                            .unwrap_or_else(|| write_ph_mnemonic_ipa(phdata, code, &nb))
                    }
                    // A tone is written through its vowel's entry as well: the
                    // vowel's declared IPA name again, else the tone's mnemonic
                    // (never the `ipa` of the tone itself).
                    Some((prev, prev_nb))
                        if phdata.get(code).is_some_and(|p| p.typ == 1 && p.program != 0)
                            && segment.get(*prev).is_some_and(|p| p.typ == 2) =>
                    {
                        program_ipa_name(segment, *prev, prev_nb).unwrap_or_else(|| {
                            let mnemonic = phdata.get(code).map_or(0, |p| p.mnemonic);
                            mnemonic_to_ipa_c(mnemonic, false)
                        })
                    }
                    _ => write_ph_mnemonic_ipa(phdata, code, &nb),
                };
                if code != PHON_SYLLABIC && code != PHON_LENGTHEN {
                    last_sound = Some((code, nb));
                }
                if let Some(sep) = sep {
                    // Not before the clause's first phoneme, not straight after
                    // a word break, and not before a modifier letter.
                    let starts_modifier = name
                        .chars()
                        .next()
                        .is_some_and(|c| matches!(c as u32, 0x2b0..=0x36f));
                    if wrote_phoneme && !after_word_break && !starts_modifier {
                        out.push(sep);
                    }
                }
                out.push_str(&std::mem::take(&mut pending_stress));
                match tie {
                    Some(t) => out.push_str(&tie_phoneme_name(&name, t)),
                    None => out.push_str(&name),
                }
                if !name.is_empty() {
                    wrote_phoneme = true;
                    after_word_break = false;
                }
            }
        }
    }

    out.push_str(&pending_stress);
    // As in `-x`: a suppressed clause boundary leaves doubled separators behind,
    // and word boundaries either side of a clause break leave stray edge spaces.
    // With a space separator the doubling is upstream's own.
    if sep != Some(' ') {
        while out.contains("  ") {
            out = out.replace("  ", " ");
        }
    }
    let _ = straight_quote_open;
    out.retain(|c| c != EMPTY_WORD_MARK);
    let kept: Vec<String> = out
        .split('\n')
        .map(|line| {
            let line = line.trim();
            match line.strip_prefix(LEADING_SPACE_MARK) {
                Some(rest) => format!(" {}", rest.trim_start()),
                None => line.to_string(),
            }
        })
        .collect();
    kept.join("\n").trim_end().to_string()
}

/// The neighbour context for the phoneme that is `real[n]`, built the same way
/// the code-list phoneme-program pass builds it.
fn neighbours_at(
    codes: &[PhonemeCode],
    real: &[usize],
    starts: &[bool],
    n: usize,
    phdata: &PhonemeData,
) -> Neighbours {
    if n == usize::MAX {
        return Neighbours::default();
    }
    let i = real[n];
    let at = |k: Option<&usize>| k.map(|&k| codes[k].code).unwrap_or(0);
    let next_wordstart = real.get(n + 1).is_some_and(|&k| {
        codes[i + 1..k]
            .iter()
            .any(|c| c.is_boundary || c.code == PHON_END_WORD)
    });
    // Stress level of the syllable this phoneme belongs to, for programs that
    // condition on it.
    let stress = syllable_stress(codes, i, phdata);
    // C keeps a lengthen mark as a flag on the vowel, so no phoneme sees one as
    // its neighbour (`;` after `i:` still follows the `i`).
    let skip_mark = codes[i].code != PHON_LENGTHEN;
    let sound = |k: &&usize| !skip_mark || codes[**k].code != PHON_LENGTHEN;
    let prev_ix = real[..n].iter().rev().find(sound);
    let mut after = real[n + 1..].iter().filter(sound);
    Neighbours {
        prev: at(prev_ix),
        this: codes[i].code,
        next: at(after.next()),
        next2: at(after.next()),
        stress,
        prev_stress: prev_ix.map_or(bytecode::STRESS_UNKNOWN, |&k| syllable_stress(codes, k, phdata)),
        this_wordstart: starts[n],
        prev_wordstart: n.checked_sub(1).is_some_and(|k| starts[k]),
        next_wordstart,
        next2_wordstart: next_wordstart
            || matches!((real.get(n + 1), real.get(n + 2)), (Some(&a), Some(&b)) if
                codes[a + 1..b].iter().any(|c| c.is_boundary || c.code == PHON_END_WORD)),
        next_vowel_word_end: next_vowel_ends_word(codes, real, n, phdata),
        ..Default::default()
    }
}

fn syllable_stress(codes: &[PhonemeCode], i: usize, phdata: &PhonemeData) -> u8 {
    codes[..i]
        .iter()
        .rev()
        .take_while(|c| !c.is_boundary)
        .find_map(|c| match c.code {
            PHON_STRESS_P | PHON_STRESS_P2 | PHON_STRESS_TONIC => Some(4u8),
            PHON_STRESS_2 | PHON_STRESS_3 => Some(3),
            c2 if matches!(phdata.get(c2), Some(p) if p.typ == 2) => Some(0),
            _ => None,
        })
        .unwrap_or(1)
}

/// Whether the next vowel of this word is its last sound: the phoneme after it
/// starts a new word or is absent (`nextVowel(isWordEnd)`).
fn next_vowel_ends_word(codes: &[PhonemeCode], real: &[usize], n: usize, phdata: &PhonemeData) -> bool {
    let crosses = |a: usize, b: usize| {
        codes[a + 1..b].iter().any(|c| c.is_boundary || c.code == PHON_END_WORD)
    };
    let mut from = real[n];
    for (k, &at) in real.iter().enumerate().skip(n + 1) {
        if crosses(from, at) {
            return false;
        }
        if phdata.get(codes[at].code).is_some_and(|p| p.typ == 2) {
            return real.get(k + 1).is_none_or(|&after| crosses(at, after));
        }
        from = at;
    }
    false
}

/// Insert `tie` before every character after the first that is a letter — C's
/// `--tie` handling in `GetTranslatedPhonemeString`.  Modifier letters and
/// combining marks (U+02B0-U+036F) never take one.
fn tie_phoneme_name(name: &str, tie: char) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_alphabetic() && !matches!(c as u32, 0x2b0..=0x36f) {
            out.push(tie);
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> Option<PhonemeData> {
        let dir = std::path::Path::new("espeak-ng-data");
        dir.join("phontab").exists().then(|| PhonemeData::load(dir).ok())?
    }

    /// The mnemonic fallback keeps a vowel's `-` (French `@-` → `ə-`) and drops
    /// a `/` variant, a leading `_`, a consonant-only `#`, and later digits.
    #[test]
    fn mnemonic_fallback_matches_c() {
        let p = |s: &str, v: bool| {
            mnemonic_to_ipa_c(crate::phoneme::PhonemeTab::pack_mnemonic(s), v)
        };
        assert_eq!(p("@-", true), "ə-");
        assert_eq!(p("a/2", true), "a");
        assert_eq!(p("_:", false), "");
        assert_eq!(p("a#", true), "a");
        assert_eq!(p("t#", false), "tʰ");
        assert_eq!(p("i2", true), "i");
        assert_eq!(p("3:", true), "ɜː");
    }

    /// Spanish `i;` carries an explicit `ipa i` in its program, so it must not
    /// come out as the mnemonic's `iʲ`.
    #[test]
    fn program_ipa_beats_the_mnemonic() {
        let Some(mut phdata) = data() else { return };
        if phdata.select_table_by_name("es").is_err() {
            return;
        }
        let code = (0..255u8)
            .find(|&c| phdata.get(c).is_some_and(|p| p.mnemonic_str() == "i;"));
        let Some(code) = code else { return };
        assert_eq!(
            write_ph_mnemonic_ipa(&phdata, code, &Neighbours::default()),
            "i"
        );
    }
}
