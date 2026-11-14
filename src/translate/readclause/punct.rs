//! Clause-end decisions of `ReadClause` (readclause.c:768-927) and `AnnouncePunctuation`.

use super::ctype::{self as ct};
use super::iface::{ReaderLang, TagProcessor};
use super::reader::{gate, ClauseReader, Scan, Step, CTRL_EMBEDDED};
use super::Terminator;

const CHAR_COMMA_BREAK: u32 = 0x557;
const ELLIPSIS: u32 = 0x2026;
const INTONATION_MASK: u32 = 0x7000;
const NAME_HU: u32 = ((b'h' as u32) << 8) + b'u' as u32;

fn is(c: u32, k: char) -> bool {
    c == k as u32
}

fn roman(c: u32) -> bool {
    matches!(char::from_u32(c), Some('I' | 'V' | 'X' | 'L'))
}

fn question_or_exclamation(t: Terminator) -> bool {
    t.bits() & (Terminator::QUESTION.bits() | Terminator::EXCLAMATION.bits()) != 0
}

impl<T: TagProcessor> ClauseReader<T> {
    pub(super) fn punct_block(&mut self, lang: &dyn ReaderLang, s: &mut Scan) -> Step {
        gate!(self.wait_after_tag(s));
        self.join_dots(s);
        s.punct_data = Terminator::NONE;
        let mut is_end = false;
        if let Some(p) = Terminator::from_codepoint(s.c1) {
            s.punct_data = p;
            self.swallow_runs(s);
            if p.has(Terminator::PUNCTUATION_IN_WORD) {
                s.stressed_word = true;
                s.tone = ((p.bits() >> 12) & 0xf) as u8;
                return Step::Skip;
            }
            is_end = ct::space(s.c2)
                || p.has(Terminator::OPTIONAL_SPACE_AFTER)
                || ct::is_bracket(s.c2)
                || is(s.c2, '?')
                || self.eof()
                || s.c2 == CTRL_EMBEDDED;
        }
        gate!(self.announce_step(lang, s, is_end));
        self.speak_name_step(lang, s);
        if is_end {
            return self.end_clause(lang, s);
        }
        Step::Go
    }

    fn wait_after_tag(&mut self, s: &mut Scan) -> Step {
        let Some(pending) = s.end_after_tag else { return Step::Go };
        if ct::space(s.c1) {
            return Step::Go;
        }
        if !ct::is_alpha(s.c1) || !ct::lower(s.c1) {
            self.ungot_char2 = s.c1;
            s.terminate(s.end_index);
            self.unget(s.c2);
            return Step::End(pending);
        }
        s.end_after_tag = None;
        Step::Go
    }

    fn join_dots(&mut self, s: &mut Scan) {
        if !(is(s.c1, '.') && is(s.c2, '.')) {
            return;
        }
        loop {
            if self.eof() {
                break;
            }
            s.c_next = self.getc();
            if !is(s.c_next, '.') {
                break;
            }
            s.c1 = ELLIPSIS;
            s.c2 = ' ' as u32;
        }
        if s.c1 == ELLIPSIS {
            s.c2 = s.c_next;
        } else {
            self.unget(s.c_next);
        }
    }

    fn swallow_runs(&mut self, s: &mut Scan) {
        if !question_or_exclamation(s.punct_data) {
            return;
        }
        while !self.eof() && Terminator::from_codepoint(s.c2).is_some_and(question_or_exclamation) {
            s.c_next = self.getc();
            s.c2 = s.c_next;
        }
    }

    fn announce_step(&mut self, lang: &dyn ReaderLang, s: &mut Scan, is_end: bool) -> Step {
        if s.c1 == 0xe000 + '<' as u32 {
            s.c1 = '<' as u32;
        }
        let listed = self.flags.punctuation == 1
            || char::from_u32(s.c1).is_some_and(|c| self.flags.punctlist.contains(&c));
        if self.flags.punctuation != 0 && ct::punct(s.c1) && !self.flags.audio_text && listed {
            if let Some(t) = self.announce_punctuation(lang, s, is_end) {
                return Step::End(t);
            }
            s.announced = s.c1;
        }
        Step::Go
    }

    fn speak_name_step(&mut self, lang: &dyn ReaderLang, s: &mut Scan) {
        if !s.punct_data.has(Terminator::SPEAK_PUNCTUATION_NAME) || s.announced != 0 {
            return;
        }
        let name = lang.lookup_char_name(s.c1, true);
        if !name.is_empty() {
            s.buf.push_str(&name);
            s.announced = s.c1;
            s.punct_data = Terminator::from_bits(s.punct_data.bits() & !INTONATION_MASK);
        }
    }

    fn end_clause(&mut self, lang: &dyn ReaderLang, s: &mut Scan) -> Step {
        let mut is_end = true;
        let mut nl_count = 0;
        s.c_next = s.c2;
        while ct::space(s.c_next) && !self.eof() {
            nl_count += u32::from(is(s.c_next, '\n'));
            s.c_next = self.getc();
        }
        if is(s.c1, '.') && nl_count < 2 {
            s.punct_data = s.punct_data.with(Terminator::DOT_AFTER_LAST_WORD);
        }
        if nl_count == 0 {
            is_end = self.refine_end(lang, s);
        }
        if !is_end {
            if !self.eof() && ct::space(s.c2) {
                self.unget(s.c_next);
            }
            return Step::Go;
        }
        s.terminate(s.buf.len());
        self.unget(s.c_next);
        if ct::digit(s.cprev) && !ct::is_alpha(s.c_next) {
            s.punct_data = s.punct_data.without(Terminator::DOT_AFTER_LAST_WORD);
        }
        Step::End(paragraph_pause(s.punct_data, nl_count))
    }

    fn refine_end(&mut self, lang: &dyn ReaderLang, s: &mut Scan) -> bool {
        let mut is_end = true;
        let (cprev, cprev2, c_next) = (s.cprev, s.cprev2, s.c_next);
        if is(s.c1, ',')
            && is(cprev, '.')
            && lang.translator_name() == NAME_HU
            && ct::digit(cprev2)
            && (ct::digit(c_next) || ct::lower(c_next))
        {
            s.c1 = CHAR_COMMA_BREAK;
            is_end = false;
        }
        if is(s.c1, '.') && is(c_next, '\'') && self.dec.peekc() == 's' as u32 {
            is_end = false;
        }
        if is(s.c1, '.') {
            is_end = self.dot_ends(lang, s, is_end);
        } else if !s.any_alnum {
            is_end = false;
        }
        if is_end && is(s.c1, '.') && is(c_next, '<') && self.opts.ssml {
            s.end_index = s.buf.len();
            s.end_after_tag = Some(s.punct_data);
            is_end = false;
        }
        is_end
    }

    fn dot_ends(&self, lang: &dyn ReaderLang, s: &mut Scan, mut is_end: bool) -> bool {
        let (cprev, cprev2, c_next) = (s.cprev, s.cprev2, s.c_next);
        let after_number = ct::digit(cprev) || (roman(cprev) && (roman(cprev2) || ct::space(cprev2)));
        if lang.ordinal_dot() && after_number && (!ct::digit(cprev) || ct::lower(c_next) || is(c_next, '-')) {
            is_end = false;
        }
        if ct::lower(c_next) && !lang.lowercase_sentence() {
            is_end = false;
        }
        if !s.any_alnum {
            s.c1 = ' ' as u32;
            is_end = false;
        }
        is_end
    }

    fn announce_punctuation(&mut self, lang: &dyn ReaderLang, s: &mut Scan, end_clause: bool) -> Option<Terminator> {
        let c1 = s.c1;
        let mut buf = String::new();
        if let Some(icon) = lang.soundicon(c1) {
            buf = format!("\u{1}{icon}I ");
            self.unget(s.c2);
        } else {
            let name = self.punct_name(lang, s, end_clause);
            if s.buf.is_empty() || !end_clause || lang.announce_punct_flag2() {
                buf = self.name_run(lang, s, &name, end_clause);
            } else {
                self.ungot_char2 = c1;
                buf.push(' ');
                self.unget(s.c2);
            }
        }
        let had_text = !s.buf.is_empty();
        s.buf.push_str(&buf);
        if !end_clause {
            return None;
        }
        Some(announce_terminator(lang, c1, had_text))
    }

    fn punct_name(&self, lang: &dyn ReaderLang, s: &Scan, end_clause: bool) -> String {
        let period = is(s.c1, '.') && end_clause && !is(s.c2, '.');
        period
            .then(|| lang.lookup_special("_.p"))
            .flatten()
            .unwrap_or_else(|| lang.lookup_char_name(s.c1, false))
    }

    fn name_run(&mut self, lang: &dyn ReaderLang, s: &mut Scan, name: &str, end_clause: bool) -> String {
        let mut count = 1;
        while !self.eof() && s.c2 == s.c1 && !is(s.c1, '<') {
            count += 1;
            s.c2 = self.getc();
        }
        if end_clause {
            self.unget(s.c2);
        }
        match count {
            1 => format!(" {name}"),
            2..=3 => repeated_names(name, count, lang.embedded_speed()),
            _ => format!(" {name} {count} {name}"),
        }
    }
}

fn repeated_names(name: &str, count: usize, speed: i32) -> String {
    let faster = speed < 300;
    let mut out = if faster { "\u{1}+10S".to_string() } else { String::new() };
    out.push_str(&format!(" {name}").repeat(count));
    if faster {
        out.push_str(" \u{1}-10S");
    }
    out
}

fn paragraph_pause(punct: Terminator, nl_count: u32) -> Terminator {
    if nl_count <= 1 {
        return punct;
    }
    if punct == Terminator::QUESTION || punct == Terminator::EXCLAMATION {
        return Terminator::from_bits(punct.bits() + 35);
    }
    Terminator::PARAGRAPH
}

fn announce_terminator(lang: &dyn ReaderLang, c1: u32, had_text: bool) -> Terminator {
    if is(c1, '-') {
        return Terminator::NONE;
    }
    let attributes = Terminator::from_codepoint(c1).unwrap_or(Terminator::NONE);
    let short = if attributes.intonation() == 1 { Terminator::SHORTCOMMA } else { Terminator::SHORTFALL };
    if had_text && !lang.announce_punct_flag2() {
        let bare = attributes.without(Terminator::OPTIONAL_SPACE_AFTER);
        return if bare == Terminator::SEMICOLON { Terminator::SHORTFALL } else { short };
    }
    if attributes.is_sentence() { attributes } else { short }
}
