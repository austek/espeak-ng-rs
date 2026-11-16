// tests/ssml_mark_events.rs
//
// MARK events of markup, against the transcripts the reference C library (f0fb11a8) prints
// (`tests/fixtures/c_events/`): a mark reports the source position of the word after it, or
// the clause's byte-index quirk when no word follows.  Other event types are covered by the
// ratchet in `c_events.rs`.

use std::path::Path;

use espeak_ng::{EspeakNg, EventKind};

fn engine(voice: &str) -> Option<EspeakNg> {
    let dir = Path::new("espeak-ng-data");
    if !dir.join("phontab").exists() {
        return None;
    }
    let mut e = EspeakNg::builder().voice(voice).data_dir(dir).build().ok()?;
    e.set_markup(true);
    Some(e)
}

/// `pos len ms sample name` of each mark event.
fn marks(text: &str) -> Option<Vec<String>> {
    let (_, _, events) = engine("en")?.synth_with_events(text).ok()?;
    Some(
        events
            .iter()
            .filter_map(|e| match &e.kind {
                EventKind::Mark(name) => {
                    Some(format!("{} {} {} {} {name}", e.text_position, e.length, e.audio_position_ms, e.sample))
                }
                _ => None,
            })
            .collect(),
    )
}

#[test]
fn mark_reports_the_next_words_position() {
    let Some(got) = marks("<speak>Hi <mark name=\"m1\"/> there.</speak>") else { return };
    assert_eq!(got, ["29 0 230 5074 m1"]);
}

#[test]
fn mark_with_no_word_after_takes_the_clause_byte_index() {
    let Some(got) = marks("hi <mark name=\"end\"/>") else { return };
    assert_eq!(got, ["9 0 331 7317 end"]);
}

#[test]
fn mark_position_counts_characters() {
    let Some(got) = marks("café <mark name=\"m\"/>über") else { return };
    assert_eq!(got, ["22 0 354 7812 m"]);
}

#[test]
fn two_marks_in_one_clause() {
    let Some(got) = marks("one <mark name=\"a\"/> two <mark name=\"b\"/> three") else { return };
    assert_eq!(got, ["22 0 226 4997 a", "43 0 435 9605 b"]);
}

#[test]
fn mark_before_the_sentence_start_comes_first() {
    let Some(got) = marks("<speak><mark name=\"s\"/>Hi there.</speak>") else { return };
    assert_eq!(got, ["24 0 0 0 s"]);
}

#[test]
fn mark_directly_before_a_sentence_end() {
    let Some(got) = marks("<s>One <mark name=\"z\"/></s><s>Two</s>") else { return };
    assert_eq!(got, ["13 0 856 18877 z"]);
}

#[test]
fn mark_in_a_nested_paragraph() {
    let Some(got) = marks("<speak><p>One <mark name=\"in\"/>two.</p> Three.</speak>") else { return };
    assert_eq!(got, ["32 0 753 16617 in"]);
}

// A clause with no word reads its mark from the dummy phoneme `TranslateWord2` adds for the
// empty word (`sourceix` 0), so the position is the clause's first character.
#[test]
fn mark_in_a_clause_without_words_takes_the_clause_start() {
    let Some(got) = marks("one, <mark name=\"end\"/>") else { return };
    assert_eq!(got, ["5 0 479 10564 end"]);
}

#[test]
fn mark_alone_is_at_the_text_start() {
    let Some(got) = marks("<mark name=\"only\"/>") else { return };
    assert_eq!(got, ["0 0 0 0 only"]);
}

#[test]
fn mark_between_blanks_is_at_the_text_start() {
    let Some(got) = marks("<speak> <mark name=\"only\"/> </speak>") else { return };
    assert_eq!(got, ["0 0 0 0 only"]);
}

#[test]
fn mark_after_a_break_takes_the_clause_start() {
    let Some(got) = marks("one <break time=\"100ms\"/> <mark name=\"end\"/>") else { return };
    assert_eq!(got, ["24 0 342 7547 end"]);
}

#[test]
fn mark_after_a_voice_takes_the_clause_start() {
    let Some(got) = marks("one <voice name=\"de\">zwei</voice> <mark name=\"end\"/>") else { return };
    assert_eq!(got, ["32 0 721 15912 end"]);
}

// The words' pre-pause phonemes carry the mark (`sourceix` 0) before the word's own first
// phoneme.
#[test]
fn mark_before_say_as_characters_sits_on_the_pre_pause() {
    let Some(got) = marks("<speak>call <mark name=\"s\"/><say-as interpret-as=\"characters\">abc</say-as> now</speak>")
    else {
        return;
    };
    assert_eq!(got, ["0 0 303 6688 s"]);
}

#[test]
fn mark_in_another_voice() {
    let Some(got) = marks("hi <voice xml:lang=\"de\">welt <mark name=\"in\"/> eins</voice> out") else { return };
    assert_eq!(got, ["48 0 615 13563 in"]);
}
