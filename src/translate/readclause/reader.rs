//! The `ReadClause` main loop (readclause.c:466-969).

use super::ctype::{self as ct, ch};
use super::decoder::Decoder;
use super::entity::parse_reference;
use super::iface::{CharMap, NoTags, ReaderFlags, ReaderLang, ReaderOptions, TagProcessor};
use super::{SourceClause, Terminator};

pub(super) const CTRL_EMBEDDED: u32 = 1;
const CHAR_EMPHASIS: u32 = 0x530;
const SAYAS_SINGLE_CHARS: u32 = 0x14;
const SAYAS_KEY: u32 = 0x24;
const N_XML_BUF: usize = 500;
const N_XML_BUF2: usize = 20;
const N_PUNCTLIST: usize = 60;
const PRIVATE_USE: u32 = 0xe000;

pub(super) enum Step {
    Go,
    Skip,
    End(Terminator),
}

macro_rules! gate {
    ($e:expr) => {
        match $e {
            Step::Go => {}
            other => return other,
        }
    };
}
pub(super) use gate;

/// Locals of one `ReadClause` call.
pub(super) struct Scan {
    pub buf: String,
    pub charix: Vec<i32>,
    pub c1: u32,
    pub c2: u32,
    pub cprev: u32,
    pub cprev2: u32,
    pub c_next: u32,
    pub linelength: i32,
    pub phoneme_mode: i32,
    pub any_alnum: bool,
    pub punct_data: Terminator,
    pub announced: u32,
    pub stressed_word: bool,
    pub end_after_tag: Option<Terminator>,
    pub end_index: usize,
    pub tone: u8,
    pub voice_change: Option<String>,
    pub upper: u32,
    pub lower: u32,
    pub clause_start: i32,
}

impl Scan {
    fn new(clause_start: i32) -> Scan {
        Scan {
            buf: String::new(),
            charix: Vec::new(),
            c1: ' ' as u32,
            c2: 0,
            cprev: ' ' as u32,
            cprev2: 0,
            c_next: 0,
            linelength: 0,
            phoneme_mode: 0,
            any_alnum: false,
            punct_data: Terminator::NONE,
            announced: 0,
            stressed_word: false,
            end_after_tag: None,
            end_index: 0,
            tone: 0,
            voice_change: None,
            upper: 0,
            lower: 0,
            clause_start,
        }
    }

    pub fn terminate(&mut self, index: usize) {
        self.buf.truncate(index);
        self.buf.push(' ');
    }

    pub fn remove_char(&mut self, index: usize) {
        let Some(c) = self.buf[index.min(self.buf.len())..].chars().next() else { return };
        let n = c.len_utf8();
        self.buf.replace_range(index..index + n, &" ".repeat(n));
    }

    fn set_charix(&mut self, index: usize, value: i32) {
        if self.charix.len() <= index {
            self.charix.resize(index + 1, 0);
        }
        self.charix[index] = value;
    }

    pub fn put(&mut self, c: u32, count: i32) {
        let before = self.buf.len();
        self.buf.push(ch(c));
        let after = self.buf.len();
        if !ct::space(c) && !ct::is_bracket(c) {
            self.set_charix(after, count - self.clause_start);
            (before + 1..after).for_each(|j| self.set_charix(j, -1));
        }
        if self.charix.len() <= after {
            self.charix.resize(after + 1, 0);
        }
    }
}

pub struct ClauseReader<T: TagProcessor = NoTags> {
    pub(super) dec: Decoder,
    pub(super) tags: T,
    pub(super) opts: ReaderOptions,
    pub(super) flags: ReaderFlags,
    pub(super) ungot_char: u32,
    pub(super) ungot_char2: u32,
    ungot_string: Vec<u32>,
    ungot_string_ix: i32,
    pub(super) count_characters: i32,
    pub(super) skipping_text: bool,
}

impl<T: TagProcessor> ClauseReader<T> {
    pub fn new(text: &str, opts: ReaderOptions, tags: T) -> Self {
        let flags = ReaderFlags {
            punctuation: opts.punctuation,
            punctlist: opts.punctlist.clone(),
            capitals: opts.capitals,
            ..ReaderFlags::default()
        };
        ClauseReader {
            dec: Decoder::new(text),
            tags,
            opts,
            flags,
            ungot_char: 0,
            ungot_char2: 0,
            ungot_string: Vec::new(),
            ungot_string_ix: -1,
            count_characters: -1,
            skipping_text: false,
        }
    }

    /// C `Eof()`.
    pub fn eof(&self) -> bool {
        self.ungot_char == 0 && self.dec.eof()
    }

    /// The test `SpeakNextClause` makes before reading another clause.
    pub fn at_end(&self) -> bool {
        self.dec.eof()
    }

    pub fn read_all(&mut self, lang: &dyn ReaderLang) -> Vec<SourceClause> {
        let mut clauses = Vec::new();
        while !self.at_end() {
            clauses.push(self.next_clause(lang));
        }
        clauses
    }

    pub(super) fn getc(&mut self) -> u32 {
        if self.ungot_char != 0 {
            return std::mem::take(&mut self.ungot_char);
        }
        self.count_characters += 1;
        self.dec.getc()
    }

    pub(super) fn unget(&mut self, c: u32) {
        self.ungot_char = c;
    }

    fn more(&self) -> bool {
        !self.eof() || self.ungot_char2 != 0 || self.ungot_string_ix >= 0
    }

    pub fn next_clause(&mut self, lang: &dyn ReaderLang) -> SourceClause {
        if self.flags.clear_skipping_text {
            self.skipping_text = false;
            self.flags.clear_skipping_text = false;
        }
        let mut s = Scan::new(self.count_characters.max(0));
        s.c2 = self.first_c2();
        while self.more() {
            if let Step::End(t) = self.iteration(lang, &mut s) {
                return self.finish(s, t);
            }
        }
        self.finish_eof(s)
    }

    fn first_c2(&mut self) -> u32 {
        if self.ungot_char2 != 0 {
            self.ungot_char2
        } else if self.eof() {
            0
        } else {
            self.getc()
        }
    }

    fn iteration(&mut self, lang: &dyn ReaderLang, s: &mut Scan) -> Step {
        gate!(self.check_limits(s));
        self.advance(s);
        gate!(self.ssml_step(s));
        if self.flags.ignore_text {
            return Step::Skip;
        }
        gate!(self.single_line(s));
        gate!(self.embedded_step(s));
        s.linelength += 1;
        match lang.ignore_or_replace(s.c1) {
            CharMap::Ignore => return Step::Skip,
            CharMap::Replace(c) => s.c1 = c,
            CharMap::Keep => {}
        }
        self.classify(lang, s);
        gate!(self.newline_step(s));
        s.announced = 0;
        if s.phoneme_mode == 0 && self.flags.sayas_mode == 0 {
            gate!(self.punct_block(lang, s));
        }
        if self.flags.silence {
            return Step::Skip;
        }
        self.emit(s)
    }

    fn check_limits(&mut self, s: &mut Scan) -> Step {
        if ct::alnum(s.c1) {
            return Step::Go;
        }
        let end = self.opts.end_character_position;
        if end > 0 && self.count_characters > end {
            s.terminate(s.buf.len());
            return Step::End(Terminator::EOF);
        }
        let skip = self.opts.skip_characters;
        if skip > 0 && self.count_characters >= skip {
            self.flags.clear_skipping_text = true;
            self.opts.skip_characters = 0;
            self.unget(s.c2);
            s.terminate(s.buf.len());
            return Step::End(Terminator::NONE);
        }
        Step::Go
    }

    fn advance(&mut self, s: &mut Scan) {
        s.cprev2 = s.cprev;
        s.cprev = s.c1;
        s.c1 = s.c2;
        if self.ungot_string_ix >= 0 && self.string_char(self.ungot_string_ix) == 0 {
            self.ungot_string_ix = -1;
        }
        if self.ungot_string_ix == 0 && self.ungot_char2 == 0 {
            s.c1 = self.string_char(0);
            self.ungot_string_ix = 1;
        }
        if self.ungot_string_ix >= 0 {
            s.c2 = self.string_char(self.ungot_string_ix);
            self.ungot_string_ix += 1;
        } else if self.eof() {
            s.c2 = ' ' as u32;
        } else {
            s.c2 = self.getc();
        }
        self.ungot_char2 = 0;
    }

    fn string_char(&self, ix: i32) -> u32 {
        self.ungot_string.get(ix as usize).copied().unwrap_or(0)
    }

    fn ssml_step(&mut self, s: &mut Scan) -> Step {
        if !self.opts.ssml || s.phoneme_mode != 0 {
            return Step::Go;
        }
        let (c1, c2) = (s.c1, s.c2);
        let lt = '<' as u32;
        if c1 == '&' as u32 && (c2 == '#' as u32 || ('a' as u32..='z' as u32).contains(&c2)) {
            self.entity_step(s);
        } else if c1 == lt && (c2 == '/' as u32 || ct::alpha(c2) || c2 == '!' as u32 || c2 == '?' as u32) {
            return self.tag_step(s);
        }
        Step::Go
    }

    fn entity_step(&mut self, s: &mut Scan) {
        let mut name: Vec<u32> = Vec::new();
        s.c1 = s.c2;
        while !self.eof() && (ct::alnum(s.c1) || s.c1 == '#' as u32) && name.len() < N_XML_BUF2 {
            name.push(s.c1);
            s.c1 = self.getc();
        }
        s.c2 = if self.eof() { 0 } else { self.getc() };
        let raw: Vec<u32> = name.iter().copied().chain([s.c1, s.c2]).take_while(|&c| c != 0).collect();
        self.ungot_string = raw;
        let parsed = if s.c1 == ';' as u32 { parse_reference(&name, s.c2) } else { None };
        match parsed {
            Some((c1, c2)) => (s.c1, s.c2) = (c1, c2),
            None => {
                self.ungot_string_ix = 0;
                (s.c1, s.c2) = ('&' as u32, ' ' as u32);
            }
        }
        if s.c1 <= 0x20 && matches!(self.flags.sayas_mode, SAYAS_SINGLE_CHARS | SAYAS_KEY) {
            s.c1 += PRIVATE_USE;
        }
    }

    fn tag_step(&mut self, s: &mut Scan) -> Step {
        if s.buf.len() > self.opts.n_buf.saturating_sub(20) {
            self.ungot_char2 = s.c1;
            s.terminate(s.buf.len());
            self.unget(s.c2);
            return Step::End(Terminator::NONE);
        }
        let mut xml: Vec<char> = Vec::new();
        s.c1 = s.c2;
        while !self.eof() && s.c1 != '>' as u32 && xml.len() < N_XML_BUF {
            xml.push(ch(s.c1));
            s.c1 = self.getc();
        }
        s.c2 = ' ' as u32;
        match self.tags.process_tag(&xml, &mut s.buf, &mut self.flags) {
            Some(t) => {
                s.terminate(s.buf.len());
                if t.is_voice_change() {
                    s.voice_change = Some(self.tags.current_voice_id());
                }
                Step::End(t)
            }
            None => {
                s.c1 = ' ' as u32;
                if !self.eof() {
                    s.c2 = self.getc();
                }
                Step::Skip
            }
        }
    }

    fn single_line(&mut self, s: &mut Scan) -> Step {
        if s.c2 != '\n' as u32 || self.opts.linelength != -1 {
            return Step::Go;
        }
        let t = Terminator::from_codepoint(s.c1).unwrap_or_else(|| {
            s.put(s.c1, self.count_characters);
            Terminator::PERIOD
        });
        s.terminate(s.buf.len());
        Step::End(t)
    }

    fn embedded_step(&mut self, s: &mut Scan) -> Step {
        if s.c1 != CTRL_EMBEDDED {
            return Step::Go;
        }
        match char::from_u32(s.c2) {
            Some('V') => {
                self.skip_voice_name(s);
                Step::End(Terminator::VOICE)
            }
            Some('B') => {
                self.punctuation_command(s);
                Step::Skip
            }
            _ => Step::Go,
        }
    }

    fn skip_voice_name(&mut self, s: &Scan) {
        let mut ix = s.buf.len() + 1;
        while !self.eof() && ix < self.opts.n_buf.saturating_sub(1) {
            if ct::space(self.getc()) {
                break;
            }
            ix += 1;
        }
    }

    fn punctuation_command(&mut self, s: &mut Scan) {
        s.buf.push_str("   ");
        let zero = !self.eof() && {
            s.c2 = self.getc();
            s.c2 == '0' as u32
        };
        if zero {
            self.flags.punctuation = 0;
        } else {
            self.flags.punctuation = 1;
            self.flags.punctlist.clear();
            if s.c2 != '1' as u32 {
                self.read_punctlist(s);
                self.flags.punctuation = 2;
            }
        }
        if !self.eof() {
            s.c2 = self.getc();
        }
    }

    fn read_punctlist(&mut self, s: &mut Scan) {
        let limit = self.opts.n_buf.saturating_sub(1);
        while !self.eof() && !ct::space(s.c2) && self.flags.punctlist.len() < N_PUNCTLIST - 1 && s.buf.len() < limit {
            self.flags.punctlist.push(ch(s.c2));
            s.c2 = self.getc();
            s.buf.push(' ');
        }
    }

    fn classify(&mut self, lang: &dyn ReaderLang, s: &mut Scan) {
        if ct::alnum(s.c1) {
            s.any_alnum = true;
        } else {
            self.fix_non_alnum(s);
        }
        if ct::upper(s.c1) {
            s.upper += 1;
            self.say_capital(lang, s);
        } else if ct::alpha(s.c1) {
            s.lower += 1;
        }
        s.phoneme_mode = self.phoneme_mode(s);
    }

    fn fix_non_alnum(&mut self, s: &mut Scan) {
        if s.stressed_word {
            s.stressed_word = false;
            s.c1 = CHAR_EMPHASIS;
            self.unget(s.c2);
            s.c2 = ' ' as u32;
        }
        if s.c1 == 0xf0b {
            s.c1 = ' ' as u32;
        }
        if s.c1 == 0xd4d && s.c2 == 0x200d {
            s.c1 = 0xd4e;
        }
        if s.c1 == 0xdca && s.c2 == 0x200d && !self.eof() {
            s.c2 = self.getc();
        }
    }

    fn say_capital(&self, lang: &dyn ReaderLang, s: &mut Scan) {
        if self.flags.capitals != 2 || self.flags.sayas_mode != 0 || ct::upper(s.cprev) {
            return;
        }
        if let Some(text) = lang.lookup_special("_cap") {
            if s.buf.len() + text.len() < self.opts.n_buf {
                s.buf.push_str(&text);
            }
        }
    }

    fn phoneme_mode(&self, s: &Scan) -> i32 {
        let (c1, c2) = (s.c1, s.c2);
        if !self.opts.phoneme_input {
            return s.phoneme_mode;
        }
        if s.phoneme_mode > 0 {
            s.phoneme_mode - 1
        } else if c1 == '[' as u32 && c2 == '[' as u32 {
            -1
        } else if c1 == ']' as u32 && c2 == ']' as u32 {
            2
        } else {
            s.phoneme_mode
        }
    }

    fn newline_step(&mut self, s: &mut Scan) -> Step {
        if s.c1 != '\n' as u32 {
            return Step::Go;
        }
        let mut parag = 0u32;
        while !self.eof() && ct::space(s.c2) {
            parag += u32::from(s.c2 == '\n' as u32);
            s.c2 = self.getc();
        }
        if parag > 0 {
            if s.end_after_tag.is_some() {
                s.remove_char(s.end_index);
            }
            s.terminate(s.buf.len());
            self.unget(s.c2);
            let parag = if self.opts.ssml { 1 } else { parag.min(3) };
            return Step::End(Terminator::from_bits(Terminator::PARAGRAPH.bits() - 30 + 30 * parag));
        }
        if s.linelength <= self.opts.linelength {
            s.terminate(s.buf.len());
            self.unget(s.c2);
            return Step::End(Terminator::COLON);
        }
        s.linelength = 0;
        Step::Go
    }

    fn emit(&mut self, s: &mut Scan) -> Step {
        if s.c1 == s.announced {
            if ct::is_bracket(s.c1) {
                s.c1 = PRIVATE_USE + '(' as u32;
            } else if s.c1 != '-' as u32 {
                s.c1 = ' ' as u32;
            }
        }
        if s.c1 == PRIVATE_USE + '<' as u32 {
            s.c1 = '<' as u32;
        }
        s.put(s.c1, self.count_characters);
        let n_buf = self.opts.n_buf;
        let ix = s.buf.len();
        if (ix > n_buf.saturating_sub(75) && !ct::is_alpha(s.c1) && !ct::digit(s.c1)) || ix >= n_buf.saturating_sub(4) {
            s.terminate(ix);
            self.unget(s.c2);
            return Step::End(Terminator::NONE);
        }
        Step::Go
    }

    fn finish_eof(&mut self, mut s: Scan) -> SourceClause {
        if s.stressed_word {
            s.put(CHAR_EMPHASIS, self.count_characters);
        }
        if s.end_after_tag.is_some() {
            s.remove_char(s.end_index);
        }
        s.terminate(s.buf.len());
        self.finish(s, Terminator::EOF)
    }

    fn finish(&mut self, s: Scan, terminator: Terminator) -> SourceClause {
        let text = s.buf.split('\0').next().unwrap_or("").to_string();
        let tone_override = (s.tone != 0).then_some(s.tone);
        SourceClause {
            text,
            charix: s.charix,
            terminator,
            tone_override,
            voice_change: s.voice_change,
            eof: self.eof(),
            upper_count: s.upper,
            lower_count: s.lower,
        }
    }
}
