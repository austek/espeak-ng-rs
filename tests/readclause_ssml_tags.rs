// ClauseReader driving the ProcessSsmlTag port. Line counts and which lines are empty were
// checked against `espeak-ng -q -m -v en -x` of C master f0fb11a8:
//   <h1>Title</h1>body                      -> "", Title, body
//   <ul><li>one</li><li>two</li></ul>       -> "", one, "", two, ""
//   Hello <audio src="nofile.wav">alt text</audio> world -> Hello, alt text, world
//   <speak>hello?</speak>                   -> hello, "", ""
//   <voice xml:lang="en">hello</voice> there -> one line
//   <speak>hello <voice xml:lang="fr">bonjour</voice> world</speak> -> hello, bonjour, world, ""

use espeak_ng::translate::readclause::ssml_tag::{SsmlEnv, SsmlState, N_SPEECH_PARAM};
use espeak_ng::translate::readclause::voice_stack::{BaseVoice, VoiceCatalog, VoiceSelect};
use espeak_ng::translate::readclause::{
    ClauseReader, DefaultLang, ReaderOptions, SsmlTags, Terminator as T,
};

struct Catalog;

impl VoiceCatalog for Catalog {
    fn by_name(&self, name: &str) -> Option<String> {
        matches!(name, "fr" | "de" | "en").then(|| name.to_string())
    }
    fn select(&self, sel: &VoiceSelect) -> Option<String> {
        let lang: String = sel.language.chars().filter(|c| *c > ' ').take(2).collect();
        Some(if lang.is_empty() { sel.name.clone() } else { lang })
    }
}

fn factors(_wpm: i32) -> (i32, i32) {
    (12, 16)
}

fn no_sound(_: &str) -> Option<i32> {
    None
}

fn clauses(text: &str) -> Vec<(String, T, Option<String>)> {
    let mut d = [0; N_SPEECH_PARAM];
    d[1] = 175;
    d[2] = 100;
    d[3] = 50;
    d[4] = 50;
    d[10] = 100;
    let base = BaseVoice {
        identifier: None,
        name: "en".into(),
        languages: vec!["en".into()],
        priority: 5,
        gender: 1,
        ..BaseVoice::default()
    };
    let env = SsmlEnv {
        voices: &Catalog,
        tone_language: false,
        n_outbuf: 800,
        pause_factors: &factors,
        load_sound: &no_sound,
        uri_callback: None,
    };
    let mut state = SsmlState::new(d, base, "");
    state.voice.current_voice_id = "en".into();
    let tags = SsmlTags::new(state, env);
    let opts = ReaderOptions { ssml: true, ..ReaderOptions::default() };
    let mut reader = ClauseReader::new(text, opts, tags);
    let all = reader.read_all(&DefaultLang);
    all.into_iter().map(|c| (c.text.trim().to_string(), c.terminator, c.voice_change)).collect()
}

fn texts(text: &str) -> Vec<String> {
    clauses(text).into_iter().map(|c| c.0).collect()
}

#[test]
fn headings_are_paragraphs() {
    assert_eq!(texts("<h1>Title</h1>body"), ["", "Title", "body"]);
}

#[test]
fn list_items_end_clauses() {
    assert_eq!(texts("<ul><li>one</li><li>two</li></ul>"), ["", "one", "", "two", ""]);
}

#[test]
fn audio_ends_a_clause_on_both_tags() {
    assert_eq!(texts("Hello <audio src=\"nofile.wav\">alt text</audio> world"), ["Hello", "alt text", "world"]);
}

#[test]
fn closing_speak_leaves_an_empty_eof_clause() {
    assert_eq!(texts("<speak>hello?</speak>"), ["hello", "", ""]);
}

#[test]
fn a_voice_of_the_same_language_changes_nothing() {
    assert_eq!(texts("<voice xml:lang=\"en\">hello</voice> there"), ["hello  there"]);
}

#[test]
fn a_voice_change_ends_the_clause_and_reports_the_new_voice() {
    let got = clauses("<speak>hello <voice xml:lang=\"fr\">bonjour</voice> world</speak>");
    let words: Vec<&str> = got.iter().map(|c| c.0.as_str()).collect();
    assert_eq!(words, ["hello", "bonjour", "world", ""]);
    assert_eq!(got[0].2.as_deref(), Some("fr"));
    assert!(got[0].1.is_voice_change());
    assert_eq!(got[1].2.as_deref(), Some("en"));
}
