//! `espeak_TextToPhonemes`: split the translation of a text into the clauses
//! C returns one call at a time.

use espeak_ng::phoneme::PhonemeData;
use espeak_ng::phoneme_text::format_mnemonics;
use espeak_ng::translate::{select_phoneme_table, Translator};

use crate::settings::Settings;

const EOF_TERMINATOR: i32 = 0x90028;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clause {
    pub phonemes: String,
    pub terminator: i32,
    pub end_byte: usize,
}

pub const PHONEMES_IPA: i32 = 0x02;
pub const PHONEMES_TIE: i32 = 0x80;

fn terminator_value(c: char) -> i32 {
    match c {
        ',' => 0x41014,
        '、' => 0x49014,
        '.' => 0x480028,
        '。' => 0x88028,
        '?' | '؟' => 0x82028,
        '!' => 0x8302d,
        ';' | '—' => 0x4101e,
        ':' => 0x4001e,
        '…' => 0x24901e,
        _ => EOF_TERMINATOR,
    }
}

/// Byte offset just after each clause-ending mark, with the mark itself.
fn terminators(text: &str) -> Vec<(char, usize)> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    chars
        .iter()
        .enumerate()
        .filter(|&(i, &(_, c))| {
            matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | '…' | '。' | '、' | '—' | '؟')
                && chars.get(i + 1).is_none_or(|&(_, n)| n.is_whitespace() || !n.is_alphanumeric())
        })
        .map(|(_, &(b, c))| (c, b + c.len_utf8()))
        .collect()
}

fn translator_for(settings: &Settings, mode: i32) -> espeak_ng::Result<Translator> {
    let mut t = Translator::new(&settings.voice, Some(&settings.data_dir))?;
    let glyph = char::from_u32(((mode >> 8) & 0xffff) as u32).filter(|&c| c != '\0');
    if mode & PHONEMES_TIE != 0 {
        t.options.ipa_tie = glyph;
    } else {
        t.options.ipa_separator = glyph;
    }
    Ok(t)
}

fn translate(settings: &Settings, text: &str, mode: i32) -> espeak_ng::Result<String> {
    let t = translator_for(settings, mode)?;
    if mode & PHONEMES_IPA != 0 {
        return t.text_to_ipa_clauses(text);
    }
    let codes = t.translate_to_clause_codes(text)?;
    let mut phdata = PhonemeData::load(&settings.data_dir)?;
    select_phoneme_table(&mut phdata, &settings.data_dir, t.options.lang.as_str())?;
    let glyph = char::from_u32(((mode >> 8) & 0xffff) as u32).filter(|&c| c != '\0');
    let (sep, tie) = if mode & PHONEMES_TIE != 0 { (None, glyph) } else { (glyph, None) };
    Ok(format_mnemonics(&codes, &phdata, sep, tie, &settings.data_dir, t.options.lang.as_str()))
}

/// The clauses of `text` in order; the last has `end_byte == text.len()`.
pub fn clauses(settings: &Settings, text: &str, mode: i32) -> espeak_ng::Result<Vec<Clause>> {
    let rendered = translate(settings, text, mode)?;
    let lines: Vec<&str> = rendered.split('\n').collect();
    let marks = terminators(text);
    let n = lines.len();
    let clauses: Vec<Clause> = lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| match (i + 1 < n, marks.get(i)) {
            (true, Some(&(c, end))) => Clause { phonemes: line.to_string(), terminator: terminator_value(c), end_byte: end },
            (true, None) => Clause {
                phonemes: line.to_string(),
                terminator: EOF_TERMINATOR,
                end_byte: spread_end(text, i, n),
            },
            (false, _) => Clause {
                phonemes: line.to_string(),
                terminator: last_terminator(text, n, &marks),
                end_byte: text.len(),
            },
        })
        .collect();
    Ok(clauses)
}

/// Where clause `i` of `n` ends when the source has no mark for it: evenly
/// spread, so that walking the clauses still visits every one.
fn spread_end(text: &str, i: usize, n: usize) -> usize {
    let mut at = (text.len() * (i + 1) / n).clamp(1, text.len().max(1));
    while !text.is_char_boundary(at.min(text.len())) {
        at += 1;
    }
    at.min(text.len().saturating_sub(1)).max(1)
}

fn closes_text(text: &str, end: usize) -> bool {
    text.get(end..)
        .is_some_and(|rest| rest.chars().all(|c| c.is_whitespace() || "\"'”’»)]".contains(c)))
}

fn last_terminator(text: &str, n: usize, marks: &[(char, usize)]) -> i32 {
    match marks.get(n - 1) {
        Some(&(c, end)) if closes_text(text, end) => terminator_value(c),
        _ => EOF_TERMINATOR,
    }
}
