// ReadClause (readclause.c:466-969) clause splits. The line count of every input was checked
// against `espeak-ng -q -m -v en -x` of C master f0fb11a8: one line per clause. Clause text is
// compared trimmed; terminators come from translate.h.

use espeak_ng::translate::readclause::{
    ClauseReader, DefaultLang, ReaderFlags, ReaderLang, ReaderOptions, SourceClause, TagProcessor,
    Terminator as T,
};

struct Tags;

impl TagProcessor for Tags {
    fn process_tag(&mut self, tag: &[char], out: &mut String, _: &mut ReaderFlags) -> Option<T> {
        let s: String = tag.iter().collect();
        let name = s.trim_start_matches('/').split([' ', '/']).next().unwrap_or("").to_string();
        let closing = s.starts_with('/');
        if name != "b" && name != "i" {
            out.push(' ');
        }
        match (name.as_str(), closing) {
            ("p", _) | ("s", false) => Some(T::PARAGRAPH),
            ("s", true) => Some(T::PERIOD),
            ("br", _) => Some(T::COLON),
            ("audio", _) => Some(T::NONE),
            _ => None,
        }
    }
    fn current_voice_id(&self) -> String {
        String::new()
    }
}

fn ssml() -> ReaderOptions {
    ReaderOptions { ssml: true, ..ReaderOptions::default() }
}

fn read(text: &str, opts: ReaderOptions) -> Vec<(String, T, bool)> {
    read_with(text, opts, &DefaultLang)
}

fn read_with(text: &str, opts: ReaderOptions, lang: &dyn ReaderLang) -> Vec<(String, T, bool)> {
    let mut reader = ClauseReader::new(text, opts, Tags);
    let clauses: Vec<SourceClause> = reader.read_all(lang);
    clauses.into_iter().map(|c| (c.text.trim().to_string(), c.terminator, c.eof)).collect()
}

fn row(text: &str, t: T) -> (String, T, bool) {
    (text.to_string(), t, false)
}

fn last(text: &str) -> (String, T, bool) {
    (text.to_string(), T::EOF, true)
}

#[test]
fn lower_case_after_a_dot_does_not_split() {
    assert_eq!(read("one. two", ssml()), vec![last("one. two")]);
}

#[test]
fn upper_case_after_a_dot_splits() {
    let dot = T::PERIOD.with(T::DOT_AFTER_LAST_WORD);
    assert_eq!(read("Hello. World", ssml()), vec![row("Hello", dot), last("World")]);
}

#[test]
fn dot_before_a_tag_waits_for_the_next_letter() {
    let dot = T::PERIOD.with(T::DOT_AFTER_LAST_WORD);
    assert_eq!(read("Hello.<emphasis>World", ssml()), vec![row("Hello", dot), last("World")]);
    assert_eq!(read("Hello.<emphasis>world", ssml()), vec![last("Hello. world")]);
    assert_eq!(read("hello. <b>World</b>", ssml()), vec![row("hello", dot), last("World")]);
    assert_eq!(read("hello.<b>world</b>", ssml()), vec![last("hello.world")]);
}

#[test]
fn dot_before_a_tag_at_the_end_is_removed() {
    assert_eq!(read("Hello.<emphasis>", ssml()), vec![last("Hello")]);
}

#[test]
fn paragraph_and_sentence_tags_end_clauses() {
    assert_eq!(
        read("a<p>b</p>c", ssml()),
        vec![row("a", T::PARAGRAPH), row("b", T::PARAGRAPH), last("c")]
    );
    assert_eq!(
        read("<s>One</s><s>Two</s>", ssml()),
        vec![row("", T::PARAGRAPH), row("One", T::PERIOD), row("", T::PARAGRAPH), row("Two", T::PERIOD), last("")]
    );
}

#[test]
fn a_tag_ending_the_text_leaves_one_empty_eof_clause() {
    assert_eq!(read("a<br/>", ssml()), vec![row("a", T::COLON), last("")]);
    assert_eq!(
        read("a<audio src=\"n.wav\">x</audio>", ssml()),
        vec![row("a", T::NONE), row("x", T::NONE), last("")]
    );
}

#[test]
fn entities_become_characters() {
    assert_eq!(read("a &#65; &#x42; b", ssml()), vec![last("a A B b")]);
    assert_eq!(read("a &amp; b", ssml()), vec![last("a & b")]);
    assert_eq!(read("a &lt; b", ssml()), vec![last("a < b")]);
}

#[test]
fn an_unknown_entity_is_read_as_text() {
    assert_eq!(read("a &bogus; b", ssml()), vec![row("a &bogus", T::SEMICOLON), last("b")]);
    assert_eq!(read("ab&", ssml()), vec![last("ab&")]);
    assert_eq!(read("a & b", ssml()), vec![last("a & b")]);
}

#[test]
fn lt_before_a_digit_is_text() {
    assert_eq!(read("a<3 b", ssml()), vec![last("a<3 b")]);
}

#[test]
fn blank_line_ends_a_paragraph() {
    assert_eq!(read("hello\n\nworld", ssml()), vec![row("hello", T::PARAGRAPH), last("world")]);
    assert_eq!(read("hello\n\nworld", ReaderOptions::default()), vec![row("hello", T::PARAGRAPH), last("world")]);
}

#[test]
fn more_blank_lines_lengthen_the_pause_outside_ssml() {
    let plain = read("a\n\n\nb", ReaderOptions::default());
    assert_eq!(plain[0].1.pause_ms(), 1000);
    assert_eq!(read("a\n\n\nb", ssml())[0].1.pause_ms(), 700);
}

#[test]
fn ellipsis_ends_a_clause_without_a_sentence() {
    let dots = T::SEMICOLON.with(T::SPEAK_PUNCTUATION_NAME).with(T::OPTIONAL_SPACE_AFTER);
    assert_eq!(read("x ... y", ssml()), vec![row("x", dots), last("y")]);
}

#[test]
fn question_exclamation_runs_use_the_first_mark() {
    assert_eq!(read("Ok?! Yes", ssml()), vec![row("Ok", T::QUESTION), last("Yes")]);
}

#[test]
fn the_decoder_sees_a_trailing_nul() {
    let mut r = ClauseReader::new("hello?", ssml(), Tags);
    let c = r.next_clause(&DefaultLang);
    assert!(c.eof && r.at_end());
    assert_eq!(c.terminator, T::QUESTION);
}

struct Names;

impl ReaderLang for Names {
    fn lookup_char_name(&self, c: u32, _only: bool) -> String {
        format!("[{}]", char::from_u32(c).unwrap_or('?'))
    }
}

#[test]
fn announced_punctuation_ends_the_clause_before_the_mark() {
    let opts = ReaderOptions { punctuation: 1, ..ssml() };
    let got = read_with("a, b.", opts, &Names);
    assert_eq!(
        got,
        vec![row("a", T::SHORTCOMMA), row("[,]", T::SHORTCOMMA), ("b".to_string(), T::SHORTFALL, true)]
    );
}
