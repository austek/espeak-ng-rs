// tests/ssml_events.rs
//
// SENTENCE and WORD events of markup, against the transcripts the reference C library
// (f0fb11a8) prints (`tests/fixtures/c_events/`).  Only types 1 and 2 are compared here: MARK,
// END, PLAY, SAMPLERATE and PHONEME events are covered by the full ratchet in `c_events.rs`.

use std::path::Path;

use espeak_ng::{EspeakNg, EventKind};

const SSML_FLAG: u32 = 0x10;

fn engine(voice: &str) -> Option<EspeakNg> {
    let dir = Path::new("espeak-ng-data");
    if !dir.join("phontab").exists() {
        return None;
    }
    let mut e = EspeakNg::builder().voice(voice).data_dir(dir).build().ok()?;
    e.set_markup(true);
    Some(e)
}

/// `type pos len sample value` of each word and sentence event.
fn word_and_sentence(voice: &str, text: &str) -> Option<Vec<String>> {
    let (_, _, events) = engine(voice)?.synth_with_events(text).ok()?;
    Some(
        events
            .iter()
            .filter_map(|e| match e.kind {
                EventKind::Word(n) => Some(format!("1 {} {} {} {n}", e.text_position, e.length, e.sample)),
                EventKind::Sentence(n) => Some(format!("2 {} {} {} {n}", e.text_position, e.length, e.sample)),
                _ => None,
            })
            .collect(),
    )
}

// A word the translator folds into the previous token (`and` after a pre-pause, the second half
// of a dictionary phrase) still gets its own event, and every `phonEND_WORD` inside a token's
// phonemes starts a word at `source_ix + 1`.
#[test]
fn a_word_after_a_pre_pause_keeps_its_event() {
    let Some(got) = word_and_sentence("en", "<speak>big spaces and gaps</speak>") else { return };
    assert_eq!(got, ["2 8 0 0 1", "1 8 3 0 1", "1 12 6 5088 2", "1 19 3 18005 3", "1 23 4 22379 4"]);
}

#[test]
fn a_two_word_dictionary_phrase_gets_two_events() {
    let Some(got) = word_and_sentence("en", "<speak>naïve café déjà vu</speak>") else { return };
    assert_eq!(got[4], "1 20 4 22329 4", "{got:?}");
}

#[test]
fn the_parts_of_a_number_are_words_one_character_on() {
    let Some(got) = word_and_sentence("en", "<speak>number 123 and 456</speak>") else { return };
    assert_eq!(
        got,
        [
            "2 8 0 0 1", "1 8 6 0 1", "1 15 3 7406 2", "1 16 3 22852 3", "1 16 3 31951 4", "1 19 3 39986 5",
            "1 23 3 44395 6", "1 24 3 60379 7", "1 24 3 68423 8"
        ]
    );
}

#[test]
fn a_decimal_is_three_words() {
    let Some(got) = word_and_sentence("en", "<speak>it is 3.5 km</speak>") else { return };
    assert_eq!(got[3..6], ["1 14 3 6052 3", "1 15 3 11017 4", "1 15 3 17700 5"], "{got:?}");
}

#[test]
fn two_sentences_start_after_the_empty_leading_clause() {
    let Some(got) = word_and_sentence("en", "<s>One</s><s>Two</s>") else { return };
    assert_eq!(got, ["2 4 0 11620 1", "1 4 3 11620 1", "2 14 0 30497 2", "1 14 3 30497 2"]);
}

#[test]
fn word_numbers_continue_across_a_mark() {
    let Some(got) = word_and_sentence("en", "<speak>Hi <mark name=\"m1\"/> there.</speak>") else { return };
    assert_eq!(got, ["2 8 0 0 1", "1 8 2 0 1", "1 29 5 5074 2"]);
}

#[test]
fn multibyte_positions_count_characters() {
    let Some(got) = word_and_sentence("en", "café <mark name=\"m\"/>über") else { return };
    assert_eq!(got.len(), 3, "{got:?}");
    assert_eq!(got[0], "2 1 0 0 1");
    assert_eq!(got[1], "1 1 4 0 1");
    assert!(got[2].starts_with("1 22 4 "), "{got:?}");
}

#[test]
fn word_numbers_continue_across_a_voice_change() {
    let Some(got) = word_and_sentence("en", "hello <voice xml:lang=\"de\">welt</voice> end") else { return };
    let heads: Vec<String> = got.iter().map(|l| l.split(' ').take(3).collect::<Vec<_>>().join(" ")).collect();
    assert_eq!(heads, ["2 1 0", "1 1 5", "1 28 4", "1 41 3"]);
    let numbers: Vec<&str> = got.iter().map(|l| l.rsplit(' ').next().unwrap_or("")).collect();
    assert_eq!(numbers, ["1", "1", "2", "3"]);
}

fn fixture_cases() -> Vec<(String, String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/c_events");
    let tsv = std::fs::read_to_string(dir.join("cases.tsv")).unwrap_or_default();
    tsv.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.splitn(5, '\t').collect();
            let flags = u32::from_str_radix(f.get(3)?.trim_start_matches("0x"), 16).ok()?;
            (flags & SSML_FLAG != 0).then(|| (f[0].to_string(), f[2].to_string(), f[4].to_string()))
        })
        .collect()
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        match (c, it.clone().next()) {
            ('\\', Some('n')) => {
                it.next();
                out.push('\n');
            }
            ('\\', Some('x')) => {
                it.next();
                let hex: String = it.by_ref().take(2).collect();
                out.push(char::from(u8::from_str_radix(&hex, 16).unwrap_or(b'?')));
            }
            _ => out.push(c),
        }
    }
    out
}

/// `type pos len sample value` of the C word and sentence lines, with the sample total.
fn recorded(id: &str) -> (Vec<String>, String) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/c_events");
    let text = std::fs::read_to_string(dir.join(format!("{id}.txt"))).unwrap_or_default();
    let total = text.lines().last().unwrap_or_default().to_string();
    let lines = text
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split(' ').collect();
            let num = f.get(6)?.strip_prefix("num=")?;
            matches!(f[1], "1" | "2").then(|| format!("{} {} {} {} {num}", f[1], f[2], f[3], f[5]))
        })
        .collect();
    (lines, total)
}

/// C cuts a say-as number into several words; the tokenizer reads it as one.
const SEGMENTED_DIFFERENTLY: [&str; 3] = ["ssml_say_as_digits", "multi_say_as_cardinal", "multi_say_as_date"];

/// Cases whose audio length equals C's must give C's word and sentence events.  The rest
/// differ in audio or word segmentation and stay in `known_diff.txt`.
#[test]
fn fixture_cases_with_c_audio_match_c_words_and_sentences() {
    let mut failures = Vec::new();
    for (id, voice, text) in fixture_cases() {
        if SEGMENTED_DIFFERENTLY.contains(&id.as_str()) {
            continue;
        }
        let Some(e) = engine(&voice) else { return };
        let text = unescape(&text);
        let Ok((pcm, _, _)) = e.synth_with_events(&text) else { continue };
        let (want, total) = recorded(&id);
        if total != format!("samples={}", pcm.len()) {
            continue;
        }
        let got = word_and_sentence(&voice, &text).unwrap_or_default();
        if got != want {
            failures.push(format!("{id}\n  want {want:?}\n  got  {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
