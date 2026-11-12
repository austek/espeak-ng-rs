//! Phoneme codes rendered as eSpeak mnemonics, the text `espeak-ng -x` writes
//! and `espeak_TextToPhonemes` returns when IPA is not requested.

use std::path::Path;

use crate::phoneme::PhonemeData;
use crate::translate::ipa_table::{
    PHON_LENGTHEN, PHON_STRESS_2, PHON_STRESS_3, PHON_STRESS_P, PHON_STRESS_P2, PHON_STRESS_PREV,
    PHON_STRESS_TONIC, PHON_STRESS_U,
};
use crate::translate::PhonemeCode;

/// Insert `tie` inside a phoneme's name, before every character after the first
/// that is a letter — C's `--tie` handling in `GetTranslatedPhonemeString`,
/// which turns `tʃ` into `t͡ʃ` and leaves `ˈiː` alone.  Modifier letters and
/// combining marks (U+02B0-U+036F) never take one.
pub fn tie_unit(unit: &str, tie: char) -> String {
    let mut out = String::with_capacity(unit.len() + 2);
    for (i, c) in unit.chars().enumerate() {
        if i > 0 && c.is_alphabetic() && !matches!(c as u32, 0x2b0..=0x36f) {
            out.push(tie);
        }
        out.push(c);
    }
    out
}

/// Render phoneme codes as eSpeak mnemonics (`-x`).
///
/// A `phonSWITCH` marker in the stream opens `(lang)` and swaps the phoneme
/// table used to resolve the mnemonics that follow, so a mid-clause language
/// switch prints `(en)h@l'oU(ru)` as upstream does instead of leaking the raw
/// marker phoneme.  Clauses are separated by `\n`.
pub fn format_mnemonics(
    codes: &[PhonemeCode],
    phdata: &PhonemeData,
    sep: Option<char>,
    tie: Option<char>,
    data_dir: &Path,
    lang: &str,
) -> String {
    let mut out = String::new();
    let mut word_units: Vec<String> = Vec::new();
    // The table the codes that follow belong to; `None` = the clause's own.
    let mut switched: Option<String> = None;
    let mut tables = crate::translate::SwitchTables::default();

    // A stress marker is written as part of the phoneme it precedes, as in C,
    // where the separator goes *before* the stress: `tS.'i:.z`, not `tS.'.i:.z`.
    let mut pending_stress = String::new();
    let mut leading_pauses = true;
    let push_unit = |u: String, word_units: &mut Vec<String>, pending: &mut String| {
        if u.is_empty() {
            return;
        }
        // `--tie` goes inside the phoneme's own name, so apply it here rather
        // than between units.
        let tied = match tie {
            Some(t) => tie_unit(&u, t),
            None => u,
        };
        word_units.push(format!("{}{}", std::mem::take(pending), tied));
    };

    let flush_word = |out: &mut String, word_units: &mut Vec<String>, sep: Option<char>| {
        if word_units.is_empty() {
            return;
        }
        let piece = match sep {
            None => word_units.concat(),
            Some(s) => word_units.join(&s.to_string()),
        };
        out.push_str(&piece);
        word_units.clear();
    };

    for pc in codes {
        if let Some(crate::translate::CodeMarker::LangSwitch(target)) = &pc.marker {
            flush_word(&mut out, &mut word_units, sep);
            // A *closing* marker binds to the word it follows — upstream writes
            // `(en)h@l'oU(ru) p@-r…`, so the word-boundary space moves after it.
            // (A switched word emits its own word boundary and the separator
            // token emits another, so there can be more than one space here.)
            let trailing_space = target == lang && out.ends_with(' ');
            while target == lang && out.ends_with(' ') {
                out.pop();
            }
            out.push_str(&format!(
                "({})",
                crate::translate::switch_marker_name(phdata, target, lang)
            ));
            if trailing_space {
                out.push(' ');
            }
            switched = (target != lang).then(|| target.to_string());
            continue;
        }
        // A target with no table keeps resolving with the clause's own.
        let phdata: &PhonemeData = switched
            .as_deref()
            .and_then(|t| tables.get(data_dir, t))
            .unwrap_or(phdata);

        if pc.is_boundary && pc.code == 15 {
            // A word boundary flushes whatever stress is pending with it.
            if !pending_stress.is_empty() {
                word_units.push(std::mem::take(&mut pending_stress));
            }
            flush_word(&mut out, &mut word_units, sep);
            out.push(' ');
            // A *space* separator is emitted at a word start too, which is why
            // C's `--sep` output has two spaces between words — but only once,
            // even where two boundaries meet (a switched word brings its own).
            if sep == Some(' ') && !out.ends_with("  ") {
                out.push(' ');
            }
            continue;
        }
        if pc.is_boundary && pc.code == 0 {
            flush_word(&mut out, &mut word_units, sep);
            if pc.clause_char.is_some() {
                leading_pauses = true;
                out.push('\n');
            } else {
                out.push(' ');
            }
            continue;
        }

        if pc.code == 15 && !pc.is_boundary {
            flush_word(&mut out, &mut word_units, sep);
            out.push(' ');
            // A *space* separator is emitted at a word start too, which is why
            // C's `--sep` output has two spaces between words — but only once,
            // even where two boundaries meet (a switched word brings its own).
            if sep == Some(' ') && !out.ends_with("  ") {
                out.push(' ');
            }
            continue;
        }

        match pc.code {
            PHON_STRESS_P | PHON_STRESS_P2 | PHON_STRESS_TONIC => pending_stress.push('\''),
            PHON_STRESS_2 | PHON_STRESS_3 => pending_stress.push(','),
            PHON_STRESS_U | PHON_STRESS_PREV => {}
            PHON_LENGTHEN => match word_units.last_mut() {
                Some(last) if pending_stress.is_empty() => last.push(':'),
                _ => push_unit(":".to_string(), &mut word_units, &mut pending_stress),
            },
            0 if !pc.is_boundary => {}
            c => {
                if let Some(ph) = phdata.get(c) {
                    if ph.typ == 1 && ph.program == 0 {
                        match ph.std_length {
                            4 => pending_stress.push('\''),
                            2 | 3 => pending_stress.push(','),
                            _ => {}
                        }
                    } else {
                        // Type-1 phonemes that carry a program are *tone*
                        // phonemes (Mandarin `55`/`214`/…), which C writes out
                        // with `WritePhMnemonic` like any other phoneme.
                        // C marks the first sound after a clause's opening
                        // pauses as a word start, so no separator precedes it.
                        let is_pause = c == crate::phoneme::PHON_PAUSE;
                        if leading_pauses && !is_pause {
                            flush_word(&mut out, &mut word_units, sep);
                        }
                        leading_pauses = leading_pauses && is_pause;
                        push_unit(ph.mnemonic_display(), &mut word_units, &mut pending_stress);
                    }
                }
            }
        }
    }

    flush_word(&mut out, &mut word_units, sep);
    // A suppressed clause boundary (a period before a lowercase word, a comma
    // between digits) leaves the surrounding separators behind; upstream prints
    // a single space there.  With a space separator the doubling is intentional.
    if sep != Some(' ') {
        while out.contains("  ") {
            out = out.replace("  ", " ");
        }
    }
    // Word boundaries either side of a clause break leave stray spaces at the
    // ends of lines (`'eI \n b'i:` where upstream writes `'eI\nb'i:`).
    let out: String = out.split('\n').map(str::trim).collect::<Vec<_>>().join("\n");
    out.trim_end().to_string()
}
