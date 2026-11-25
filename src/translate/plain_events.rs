//! Plain text with the positions and counts C's `TranslateClause` leaves for `Generate`.
//!
//! The translator reads the whole text at once, so the clauses are cut from its code stream
//! and given the message counts the clause reader would have kept.

use super::readclause::{ClauseReader, NoTags, ReaderOptions, SourceClause};
use super::c_words::{apply_cut_words, CutWord};
use super::ssml_driver::ClauseLang;
use super::{
    punctuation_argument, split_after_sentences, CTRL_EMBEDDED, PUNCTUATION_COMMAND_BLANKS, PhonemeCode, SourceIx, TokenizeMode, Translator, SOURCE_IX_MASK, TRANSLATE_PIECE_BYTES,
};
use crate::error::Result;
use crate::phoneme::PHON_END_WORD;
use crate::synthesize::cgen::phlist::{split_clauses, ClauseCodes, ClauseCounters};

impl Translator {
    /// The clauses of plain `text`, each with the counters C reads while generating it.
    pub(crate) fn translate_plain_clauses(&self, text: &str) -> Result<Vec<ClauseCodes>> {
        let mut codes = Vec::new();
        let mut base = 0;
        for piece in split_after_sentences(text, TRANSLATE_PIECE_BYTES) {
            let (source, charix) = plain_source(piece, base);
            base += piece.chars().count();
            codes.extend(self.translate_codes_in_mode(&source, true, None, TokenizeMode::Plain, None, Some(&charix))?);
        }
        Ok(plain_clauses(&codes, text, &self.plain_reads(text)))
    }

    /// What `ReadClause` and `TranslateClause` make of each clause of plain `text`.
    fn plain_reads(&self, text: &str) -> Vec<ClauseRead> {
        let options = ReaderOptions { ssml: false, ..self.reader_options() };
        let mut reader = ClauseReader::new(text, options, NoTags);
        let clauses = reader.read_all(&ClauseLang(self));
        let read = |c: &SourceClause| ClauseRead {
            start_char: c.start_char.max(0) as u32,
            end_char: c.end_char.max(0) as u32,
            blank: c.text.chars().all(|ch| ch <= ' '),
            cut: self.cut_clause_words(&c.text, &c.charix, 0, 0),
        };
        clauses.iter().map(read).collect()
    }
}

/// A clause as the reader and the word cutter left it.
struct ClauseRead {
    start_char: u32,
    end_char: u32,
    blank: bool,
    cut: Vec<CutWord>,
}

/// The text the reader hands to `TranslateClause` and its `charix`: the 1-based character index
/// of each byte (`-1` inside a multibyte character), `0` on blanks.  `\x01B` and its argument
/// become blanks, and the blank that ends a punctuation list is dropped.
fn plain_source(piece: &str, base: usize) -> (String, Vec<i32>) {
    let mut source = (String::with_capacity(piece.len()), Vec::with_capacity(piece.len()));
    let mut chars = piece.chars().peekable();
    let mut read = 0;
    while let Some(c) = chars.next() {
        read += 1;
        if c == CTRL_EMBEDDED && chars.next_if_eq(&'B').is_some() {
            let (listed, consumed) = punctuation_argument(&mut chars);
            read += 1 + consumed;
            push_blanks(&mut source, PUNCTUATION_COMMAND_BLANKS + listed);
        } else {
            push_char(&mut source, c, (base + read) as i32);
        }
    }
    source
}

fn push_blanks((text, charix): &mut (String, Vec<i32>), count: usize) {
    text.extend(std::iter::repeat_n(' ', count));
    charix.extend(std::iter::repeat_n(0, count));
}

fn push_char((text, charix): &mut (String, Vec<i32>), c: char, index: i32) {
    text.push(c);
    charix.push(if c.is_whitespace() { 0 } else { index });
    charix.extend(std::iter::repeat_n(-1, c.len_utf8() - 1));
}

/// `count_sentences`, `count_words` and `new_sentence` between clauses.
struct Tally {
    sentences: u32,
    words: u32,
    new_sentence: bool,
    start_char: u32,
}

/// Where a clause's closing mark left the reader: its `count_characters` and whether the
/// character after the mark was read; `None` when the clause ran to the end of the text.
fn closing_mark(code: &PhonemeCode) -> Option<(u32, bool)> {
    code.source.map(|s| (s.ix, s.len != 0))
}

fn closes_clause(code: &PhonemeCode) -> bool {
    code.code == 0 && code.is_boundary && code.marker.is_none()
}

fn plain_clauses(codes: &[PhonemeCode], text: &str, reads: &[ClauseRead]) -> Vec<ClauseCodes> {
    let closings: Vec<Option<(u32, bool)>> = codes.iter().filter(|c| closes_clause(c)).map(closing_mark).collect();
    let marks: Vec<Option<char>> = codes.iter().filter(|c| closes_clause(c)).map(|c| c.clause_char).collect();
    let bytes: Vec<usize> = text.char_indices().map(|(i, _)| i).chain(std::iter::once(text.len())).collect();
    let total = bytes.len() as u32 - 1;
    let mut tally = Tally { sentences: 0, words: 0, new_sentence: true, start_char: 0 };
    let mut clauses = split_clauses(codes);
    let last = clauses.len() - 1;
    for (i, clause) in clauses.iter_mut().enumerate() {
        let closing = closings.get(i).copied();
        let read = (reads.len() == last + 1).then(|| &reads[i]);
        if let Some(r) = read {
            tally.start_char = r.start_char.min(total);
        }
        let (end, lookahead) = match (read, closing) {
            (Some(r), _) => (r.end_char, false),
            (None, Some(Some(mark))) => mark,
            (None, Some(None)) if i < last => (tally.start_char, false),
            _ => (total, false),
        };
        let end = end.clamp(tally.start_char, total);
        let span_end = end - u32::from(lookahead && end > tally.start_char);
        let span = &text[bytes[tally.start_char as usize]..bytes[span_end as usize]];
        let span = if read.is_some() { span.trim_end_matches(char::is_whitespace) } else { span };
        let blank = read.map_or_else(|| span.chars().all(char::is_whitespace), |r| r.blank);
        let opens = tally.new_sentence && !blank;
        tally.sentences += u32::from(opens);
        let eof_pad = usize::from(closing.is_none());
        clause.counters = Some(ClauseCounters {
            start_char: tally.start_char,
            start_word: tally.words + 1,
            sentences: tally.sentences,
            new_sentence: opens,
            end_char: end,
            end_ix: ((span.len() - unwritten_mark_bytes(span, marks.get(i).copied().flatten()) + 1 + eof_pad) & 0x7ff) as u16,
        });
        tally.words += read.map_or_else(|| clause.codes.iter().filter(|c| c.source.is_some()).count() as u32, |r| r.cut.len() as u32);
        tally.new_sentence = clause.terminator.is_sentence() || (blank && tally.new_sentence);
        relocate(&mut clause.codes, tally.start_char, read.map_or(&[], |r| r.cut.as_slice()));
        tally.start_char = end;
    }
    clauses
}

/// `ReadClause` ends the clause before it writes the closing mark, so a mark of more than one
/// byte leaves its extra bytes out of the clause buffer that `source_index` counts.
fn unwritten_mark_bytes(span: &str, mark: Option<char>) -> usize {
    mark.filter(|m| span.ends_with(*m)).map_or(0, |m| m.len_utf8() - 1)
}

/// Makes the word positions relative to the clause start, as the reader's `charix` is, and
/// gives each word a `||` in its pronunciation splits off the position after its own.
fn relocate(codes: &mut [PhonemeCode], start_char: u32, cut: &[CutWord]) {
    for code in codes.iter_mut() {
        if let Some(s) = code.source.as_mut() {
            s.ix = s.ix.saturating_sub(start_char);
        }
    }
    apply_cut_words(codes, cut);
    let mut word: Option<SourceIx> = None;
    for at in 0..codes.len() {
        match codes[at].source {
            Some(s) => word = Some(s),
            None if ends_word(&codes[at]) && !word_follows(&codes[at + 1..]) => {
                codes[at].source = word.map(next_position);
            }
            None => {}
        }
    }
}

/// A `||` in a pronunciation, or the boundary of a word.
fn ends_word(code: &PhonemeCode) -> bool {
    code.code == PHON_END_WORD || (code.is_boundary && code.code == 0)
}

/// Whether the next sounding code after a word end starts a word of its own.  A pause that
/// opens the word (a clause mark read as a word begins with one) is not what it starts at.
fn word_follows(rest: &[PhonemeCode]) -> bool {
    rest.iter()
        .find(|c| c.source.is_some() || !(ends_word(c) || c.marker.is_some()))
        .is_some_and(|c| c.source.is_some())
}

/// `srcix = source_ix + 1` of `TranslateWord2` for a `||` in the pronunciation: the packed
/// index and length of the word, one up.
fn next_position(word: SourceIx) -> SourceIx {
    let packed = (word.ix as i32 & SOURCE_IX_MASK) | (i32::from(word.len) << 11);
    let next = packed + 1;
    SourceIx { ix: (next & SOURCE_IX_MASK) as u32, len: ((next >> 11) & 0x1f) as u8 }
}
