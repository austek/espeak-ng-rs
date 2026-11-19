//! The words C's `TranslateClause` cuts from a clause, as far as their number and their source
//! positions (`words[].sourceix`, `words[].length`) go.
//!
//! C cuts the clause one character at a time and inserts spaces where a word ends (a letter
//! after a digit, punctuation after a letter, a capital after a lower-case letter, ...).  The
//! tokenizer cuts differently, so this walks the same characters the way C does and only keeps
//! what the events need: how many words C made, and where each one starts and how long it is.

use super::letters;
use super::ucd_alpha::{is_alpha, is_emoji};

const N_CLAUSE_WORDS: usize = 300;
const N_TR_SOURCE: usize = 800;
const SAYAS_KEY: i32 = 0x24;
const SAYAS_SINGLE_CHARS: i32 = 0x14;
const SAYAS_DIGITS: i32 = 0x40;
const CHAR_EMPHASIS: u32 = 0x0530;
const CHAR_COMMA_BREAK: u32 = 0x0557;
const EMBEDDED_COMMANDS: &str = "PSARHTIVYMUBF";

/// The voice settings `TranslateClause` reads while cutting.
#[derive(Clone, Debug)]
pub(crate) struct CutOptions {
    pub lang: String,
    pub letter_bits_offset: u32,
    pub tone_numbers: bool,
    pub caps_in_word: bool,
    pub caps_word_split: bool,
    pub apostrophe: u8,
    pub decimal_sep: char,
    pub dutch_ij: bool,
}

/// One word of `words[]`: the clause character it counts from and its length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CutWord {
    pub sourceix: i32,
    pub length: usize,
    /// The reader's `charix` of the character the word's text starts with, `0` when that is
    /// a bracket or blank.
    pub first: i32,
}

/// The word being cut: where it starts in `sbuf` and its position, length and first
/// character in the source.
#[derive(Clone, Copy, Default)]
struct Open {
    start: usize,
    sourceix: i32,
    length: usize,
    first: Option<i32>,
}

struct Cut<'a> {
    o: &'a CutOptions,
    source: Vec<u8>,
    charix: Vec<i32>,
    top: usize,
    sayas: i32,
    sayas_digits: usize,
    embedded_count: usize,
    words: Vec<CutWord>,
    nospace: Vec<bool>,
}

/// The words C cuts from `text` (the clause as the reader left it, closing space included),
/// with the reader's `charix` (the value of a character sits at the byte after it).
pub(crate) fn cut_words(text: &str, charix: &[i32], sayas: i32, sayas_digits: usize, o: &CutOptions) -> Vec<CutWord> {
    let top = text.len().saturating_sub(1);
    let mut ix = charix.to_vec();
    ix.resize(top + 4, 0);
    ix[top + 1] = 0;
    ix[top + 2] = 0x7fff;
    ix[top + 3] = 0;
    let mut cut = Cut {
        o,
        source: text.bytes().chain([0, 0, 0, 0]).collect(),
        charix: ix,
        top,
        sayas,
        sayas_digits,
        embedded_count: 0,
        words: Vec::new(),
        nospace: Vec::new(),
    };
    cut.run();
    cut.words
}

impl super::Translator {
    /// The words C cuts from the clause `text` read with `charix`, in the say-as state it starts in.
    pub(crate) fn cut_clause_words(&self, text: &str, charix: &[i32], sayas: i32, sayas_digits: usize) -> Vec<CutWord> {
        let g = &self.options.number_grammar;
        let options = CutOptions {
            lang: super::primary_bcp47_subtag(&self.options.lang).to_string(),
            letter_bits_offset: self.options.letter_bits_offset,
            tone_numbers: g.tone_numbers,
            caps_in_word: g.caps_mark_stress,
            caps_word_split: g.caps_word_split,
            apostrophe: g.apostrophe,
            decimal_sep: g.decimal_separator,
            dutch_ij: g.dutch_ij,
        };
        cut_words(text, charix, sayas, sayas_digits, &options)
    }
}

/// Gives each word start the position and length C cut for the word that starts there.
pub(crate) fn apply_cut_words(codes: &mut [super::PhonemeCode], cut: &[CutWord]) {
    for code in codes.iter_mut() {
        let Some(source) = code.source.as_mut() else { continue };
        if let Some(word) = cut.iter().find(|w| w.first > 0 && w.first as u32 == source.ix) {
            *source = super::SourceIx { ix: word.sourceix.max(0) as u32, len: word.length.min(31) as u8 };
        }
    }
}

fn is_digit09(c: u32) -> bool {
    (0x30..=0x39).contains(&c)
}

fn is_digit(c: u32) -> bool {
    char::from_u32(c).is_some_and(|c| c.is_ascii_digit() || (!c.is_ascii() && c.is_numeric()))
}

fn upper(c: u32) -> bool {
    char::from_u32(c).is_some_and(char::is_uppercase)
}

fn lower(c: u32) -> bool {
    char::from_u32(c).is_some_and(char::is_lowercase)
}

fn alpha(c: u32) -> bool {
    char::from_u32(c).is_some_and(is_alpha)
}

fn emoji(c: u32) -> bool {
    char::from_u32(c).is_some_and(is_emoji)
}

fn bracket(c: u32) -> bool {
    c == 0xe000 + u32::from('<') || char::from_u32(c).is_some_and(super::is_bracket)
}

fn iswspace(c: u32) -> bool {
    char::from_u32(c).is_some_and(super::readclause::attrs::is_space)
}

fn is_space(c: u32) -> bool {
    c != 0 && ((0x2500..0x25a0).contains(&c) || (0xfff9..=0xffff).contains(&c) || iswspace(c))
}

fn lowered(c: u32) -> u32 {
    char::from_u32(c).and_then(|c| c.to_lowercase().next()).map_or(c, u32::from)
}

/// C's `TranslateChar` as far as it changes how a word is cut: Hangul syllables split into jamo.
fn translate_char(c: u32) -> (u32, u32) {
    const COMPAT: [u8; 0x34] = [
        0, 0x00, 0x01, 0xaa, 0x02, 0xac, 0xad, 0x03, 0x04, 0x05, 0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb4, 0xb6, 0x06, 0x07,
        0x08, 0xb9, 0x09, 0x0a, 0xbc, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
        0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75,
    ];
    if (0xac00..=0xd7af).contains(&c) {
        let code = c - 0xac00;
        let (initial, medial, last) = ((code / 28) / 21, (code / 28) % 21, code % 28);
        return if initial == 11 {
            (medial + 0x1161, if last > 0 { last + 0x11a7 } else { 0 })
        } else {
            (initial + 0x1100, (11 * 28 * 21) + (medial * 28) + last + 0xac00)
        };
    }
    match c.checked_sub(0x3130) {
        Some(code) if (code as usize) < COMPAT.len() => (u32::from(COMPAT[code as usize]) + 0x1100, 0),
        _ => (c, 0),
    }
}

impl Cut<'_> {
    fn at(&self, j: usize) -> i32 {
        self.charix.get(j).copied().unwrap_or(0)
    }

    fn byte(&self, j: usize) -> u8 {
        self.source.get(j).copied().unwrap_or(0)
    }

    /// `utf8_in`: the character at byte `j` and its length.
    fn char_at(&self, j: usize) -> (u32, usize) {
        let rest = &self.source[j.min(self.source.len())..];
        let n = match rest.first() {
            Some(b) if *b >= 0xf0 => 4,
            Some(b) if *b >= 0xe0 => 3,
            Some(b) if *b >= 0xc0 => 2,
            _ => 1,
        };
        match rest.get(..n).and_then(|b| std::str::from_utf8(b).ok()).and_then(|s| s.chars().next()) {
            Some(c) => (u32::from(c), n),
            None => (u32::from(rest.first().copied().unwrap_or(0)), 1),
        }
    }

    /// `utf8_in2(.., 1)`: the character that ends just before byte `j`.
    fn char_before(&self, j: usize) -> u32 {
        let mut from = j.saturating_sub(1);
        while from > 0 && self.byte(from) & 0xc0 == 0x80 {
            from -= 1;
        }
        self.char_at(from).0
    }

    /// `CalcWordLength`: the position of the word at or after `from` and its length.
    fn word_length(&self, from: usize) -> (i32, usize) {
        let mut j = from;
        while j < self.top && self.at(j) <= 0 {
            j += 1;
        }
        let sourceix = self.at(j);
        let mut length = 0;
        while self.at(j) != 0 {
            length += usize::from(self.at(j) != -1);
            j += 1;
        }
        (sourceix, length)
    }

    /// `EmbeddedCommand`: the index after the command at `from`, or `None` when it is no command.
    fn embedded_command(&mut self, from: usize) -> Option<usize> {
        let mut at = from;
        if matches!(self.byte(at), b'+' | b'-') {
            at += 1;
        }
        let digits = self.source[at.min(self.source.len())..].iter().take_while(|b| b.is_ascii_digit()).count();
        let value = std::str::from_utf8(&self.source[at..at + digits]).ok().and_then(|s| s.parse::<i32>().ok());
        at += digits;
        let letter = self.byte(at);
        at += 1;
        let cmd = EMBEDDED_COMMANDS.find(char::from(letter)).filter(|_| letter != 0)?;
        if cmd + 1 == 9 {
            self.sayas = value.unwrap_or(0);
            self.sayas_digits = 0;
        }
        Some(at)
    }

    fn run(&mut self) {
        let o = self.o;
        let mut sbuf: Vec<u32> = vec![0, 0x20, 0x20];
        let (mut source_index, mut source_index_word) = (0usize, 0usize);
        let (mut prev_in, mut prev_out, mut prev_in_save) = (0x20u32, 0x20u32, 0u32);
        let mut char_inserted = 0u32;
        let (mut letter_count, mut alpha_count) = (0usize, 0usize);
        let mut all_upper = true;
        let mut other_flags = false;
        let (mut space_inserted, mut single_quoted, mut decimal_sep_count) = (false, false, false);
        let (mut phoneme_mode, mut syllable_marked, mut finished) = (false, false, false);
        let (mut word_nospace, mut next_nospace) = (false, false);
        let (sourceix, length) = self.word_length(0);
        let mut cur = Open { start: sbuf.len(), sourceix, length, first: None };
        let tone_numbers = o.tone_numbers;
        let mut cc = 0u32;
        while !finished && sbuf.len() < N_TR_SOURCE - 1 {
            let prev_out2 = prev_out;
            prev_out = *sbuf.last().unwrap_or(&0x20);
            if tone_numbers && is_digit09(prev_out) && alpha(prev_out2) {
                prev_out = u32::from('a');
            }
            if prev_in_save != 0 {
                prev_in = prev_in_save;
                prev_in_save = 0;
            } else if source_index > 0 {
                prev_in = self.char_before(source_index);
            }
            let prev_source_index = source_index;
            let mut c;
            let inserted = char_inserted != 0;
            if inserted {
                c = char_inserted;
                char_inserted = 0;
            } else {
                let (ch, n) = self.char_at(source_index);
                source_index += n;
                cc = ch;
                c = ch;
            }
            let (mut next_in, next_in_nbytes);
            if c == 0 {
                finished = true;
                c = 0x20;
                next_in = 0x20;
                next_in_nbytes = 0;
            } else {
                let (n, nb) = self.char_at(source_index);
                next_in = n;
                next_in_nbytes = nb;
            }
            if c == 1 {
                let srcix = source_index - 1;
                if prev_in != 0x20 {
                    c = 0x20;
                    prev_in_save = c;
                    source_index -= 1;
                } else {
                    if let Some(end) = self.embedded_command(source_index) {
                        self.embedded_count += 1;
                        source_index = end;
                    }
                    prev_in_save = prev_in;
                    (srcix..source_index).for_each(|j| self.source[j] = b' ');
                    source_index = srcix;
                    continue;
                }
            }
            if self.sayas == SAYAS_KEY && c != 0x20 {
                if prev_in == 0x20 && next_in == 0x20 {
                    self.sayas = SAYAS_SINGLE_CHARS;
                }
                c = lowered(c);
            }
            if phoneme_mode {
                all_upper = false;
                other_flags = true;
                if c == u32::from(']') && next_in == u32::from(']') {
                    phoneme_mode = false;
                    source_index += 1;
                    c = 0x20;
                }
            } else if (self.sayas & 0xf0) == SAYAS_DIGITS {
                if is_digit09(c) {
                    self.sayas_digits += 1;
                    if self.sayas_digits > (self.sayas & 0xf) as usize {
                        c = 0x20;
                        space_inserted = true;
                        self.sayas_digits = 0;
                    }
                } else {
                    self.sayas_digits = 0;
                    if is_digit09(prev_out) {
                        c = 0x20;
                        space_inserted = true;
                    }
                }
            } else if self.sayas & 0x10 == 0 {
                if matches!(c, 0x92 | 0xb4 | 0x2019 | 0x2032) {
                    c = u32::from('\'');
                }
                if (c == 0x2018 || c == u32::from('?')) && alpha(prev_out) && alpha(next_in) {
                    c = u32::from('\'');
                }
                if c == CHAR_EMPHASIS || c == CHAR_COMMA_BREAK {
                    c = 0x20;
                }
                let (tc, insert) = translate_char(c);
                c = tc;
                if insert != 0 {
                    char_inserted = insert;
                }
                if c == 8 {
                    continue;
                }
                if c == 0xfe0e || c == 0xfe0f {
                    continue;
                }
                if char_inserted != 0 {
                    next_in = char_inserted;
                }
                let within = |x: u32| x == u32::from('\'') || x == 0x2019 || (o.lang == "ca" && x == 0xb7);
                if !alpha(c) && !is_space(c) && !within(c) && alpha(prev_out) {
                    let tone_digit = tone_numbers && is_digit09(c) && !is_digit09(next_in);
                    let zwj = c == 0x200d && emoji(prev_out) && emoji(next_in);
                    let tag = (0xe0020..=0xe007f).contains(&c) && emoji(prev_out);
                    if !(tone_digit || zwj || tag) {
                        c = 0x20;
                        space_inserted = true;
                    }
                }
                if is_digit09(prev_out) {
                    if !is_digit09(c) && c != u32::from('.') && c != u32::from(',') && c != 0x20 {
                        c = 0x20;
                        space_inserted = true;
                    }
                } else if prev_in == u32::from(',') {
                    if c == u32::from(',') {
                        c = 0x20;
                    }
                } else {
                    decimal_sep_count = false;
                }
                if c == u32::from('[') && next_in == 2 {
                    phoneme_mode = true;
                    source_index += 1;
                    continue;
                }
                if alpha(c) {
                    alpha_count += 1;
                    let ri = |x: u32| (0x1f1e6..=0x1f1ff).contains(&x);
                    let (mut emoji_join, mut emoji_break) = (false, false);
                    if prev_out == 0x200d && emoji(c) {
                        emoji_join = emoji(sbuf.get(cur.start).copied().unwrap_or(0));
                    } else if emoji(c) && emoji(prev_out) {
                        if (0x1f3fb..=0x1f3ff).contains(&c) {
                        } else if ri(c) && ri(prev_out) {
                            let n_ri = sbuf[cur.start.min(sbuf.len())..].iter().rev().take_while(|&&x| ri(x)).count();
                            emoji_break = n_ri % 2 == 0;
                        } else {
                            emoji_break = true;
                        }
                    }
                    let foreign = |x: u32| {
                        letters::alphabet_from_char(x).is_some_and(|a| a.offset != o.letter_bits_offset)
                    };
                    let split_ideograph = tone_numbers && (c > 0x3040 || prev_out > 0x3040);
                    if emoji_join {
                    } else if emoji_break
                        || !alpha(prev_out)
                        || split_ideograph
                        || emoji(c) != emoji(prev_out)
                        || foreign(c) != foreign(prev_out)
                    {
                        if !within(prev_out) {
                            letter_count = 0;
                        }
                        if prev_out != 0x20 && !within(prev_out) {
                            c = 0x20;
                            space_inserted = true;
                            if !bracket(prev_out) {
                                next_nospace = true;
                            }
                        } else if prev_out == 0x20 && sbuf.len() >= 2 && is_digit09(sbuf[sbuf.len() - 2]) && !is_digit09(prev_in) {
                            sbuf.push(0x20);
                            cur.start += 1;
                        }
                    }
                    if c != 0x20 {
                        letter_count += 1;
                        let off = o.letter_bits_offset;
                        if off > 0
                            && ((c < 0x250 && prev_out >= off) || (c >= off && letter_count > 1 && prev_out < 0x250))
                            && alpha(prev_out)
                        {
                            c = 0x20;
                            space_inserted = true;
                        }
                    }
                    if upper(c) {
                        c = lowered(c);
                        if o.caps_in_word {
                            if !syllable_marked {
                                char_inserted = c;
                                c = 0x2c8;
                                syllable_marked = true;
                            }
                        } else if lower(prev_in) {
                            if prev_out != 0x20 && o.caps_word_split {
                                c = 0x20;
                                space_inserted = true;
                                prev_in_save = c;
                            }
                        } else if c != 0x20 && upper(prev_in) && lower(next_in) {
                            let (next2, _) = self.char_at(source_index + next_in_nbytes);
                            let dutch = o.dutch_ij && letter_count == 2 && c == u32::from('j') && prev_in == u32::from('I');
                            if !dutch && prev_out != 0x20 && alpha(next2) {
                                c = 0x20;
                                space_inserted = true;
                                prev_in_save = c;
                                next_nospace = true;
                            }
                        }
                    } else if (all_upper || other_flags) && letter_count > 2 {
                        if o.lang == "en" && c == u32::from('s') && next_in == 0x20 {
                            c = 0x20;
                            other_flags = true;
                            if sbuf.last() == Some(&u32::from('\'')) {
                                *sbuf.last_mut().unwrap_or(&mut 0) = 0x20;
                            }
                        } else {
                            all_upper = false;
                            other_flags = false;
                        }
                    } else {
                        all_upper = false;
                        other_flags = false;
                    }
                } else if c == u32::from('-') {
                    if !is_space(prev_in) && alpha(next_in) {
                        c = 0x20;
                        space_inserted |= prev_out != 0x20;
                    } else if prev_in == 0x20 && next_in == 0x20 {
                        c = 0x20;
                    } else if next_in == u32::from('-') {
                        source_index += 1;
                        c = 0x20;
                    } else if prev_out == 0x20 && alpha(prev_out2) && !alpha(prev_in) {
                        sbuf.push(0x20);
                        cur.start += 1;
                    }
                } else if c == u32::from('.') {
                    if prev_out == u32::from('.') {
                        c = 0x20;
                        space_inserted = true;
                    } else if !self.words.is_empty()
                        && !self.nospace.last().copied().unwrap_or(false)
                        && alpha(prev_in)
                        && (is_space(next_in) || next_in == u32::from('-'))
                    {
                        c = 0x20;
                    }
                } else if c == u32::from('\'') {
                    let in_word = ((prev_in == u32::from('.') && next_in == u32::from('s'))
                        || char::from_u32(prev_in).is_some_and(char::is_alphanumeric))
                        && alpha(next_in);
                    let kept = (o.apostrophe & 1 != 0 && alpha(next_in)) || (o.apostrophe & 2 != 0 && alpha(prev_in));
                    if in_word || kept {
                        single_quoted = false;
                    } else if prev_out == u32::from('s') && !single_quoted {
                        c = 0x20;
                    } else {
                        single_quoted = is_space(prev_out);
                        c = 0x20;
                    }
                } else if c == u32::from('_') {
                    c = 0x20;
                } else if is_digit09(c) {
                    if tone_numbers && alpha(prev_out) && !is_digit(next_in) {
                    } else if prev_out != 0x20 && !is_digit09(prev_out) {
                        let sep = u32::from(o.decimal_sep);
                        if prev_out != sep || (decimal_sep_count && o.decimal_sep == ',') {
                            c = 0x20;
                            space_inserted = true;
                        } else {
                            decimal_sep_count = true;
                        }
                    } else if prev_out == 0x20 && alpha(prev_out2) && !alpha(prev_in) {
                        sbuf.push(0x20);
                        cur.start += 1;
                    }
                }
            }
            if is_space(c) {
                if prev_out == 0x20 {
                    continue;
                }
                if space_inserted {
                    let mut j = 0;
                    let mut k = source_index as isize - 1;
                    while k >= source_index_word as isize && self.at(k as usize) != 0 {
                        j += usize::from(self.at(k as usize) > 0);
                        k -= 1;
                    }
                    cur.length = j;
                }
                source_index_word = source_index;
                sbuf.push(0x20);
                if self.words.len() < N_CLAUSE_WORDS - 1 && sbuf.len() > cur.start {
                    self.embedded_count = 0;
                    self.words.push(CutWord { sourceix: cur.sourceix, length: cur.length, first: cur.first.unwrap_or(0) });
                    self.nospace.push(word_nospace);
                    let (sourceix, length) = self.word_length(source_index);
                    cur = Open { start: sbuf.len(), sourceix, length, first: None };
                    word_nospace = next_nospace;
                    next_nospace = false;
                    all_upper = true;
                    other_flags = false;
                    alpha_count = 0;
                    syllable_marked = false;
                }
                if space_inserted {
                    source_index = prev_source_index;
                    char_inserted = 0;
                    space_inserted = false;
                }
            } else if sbuf.len() < N_TR_SOURCE - 4 {
                sbuf.push(c);
                if cur.first.is_none() {
                    cur.first = Some(if inserted { 0 } else { self.at(source_index) });
                }
            }
        }
        let _ = (alpha_count, cc);
        if self.words.is_empty() && self.embedded_count > 0 {
            self.words.push(CutWord { sourceix: 0, length: 0, first: 0 });
        }
    }
}
