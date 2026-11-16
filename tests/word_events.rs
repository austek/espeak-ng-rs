// tests/word_events.rs
//
// Word/sentence boundary events (`EspeakNg::synth_with_events`): each spoken
// word gets a `Word` event carrying its start time in the audio and a
// best-effort source `text_position`, bracketed by `Sentence` and
// `End`/`MsgTerminated`.  Skips when the local espeak-ng data is absent.

use std::path::Path;

use espeak_ng::{EspeakNg, EventKind};

fn engine(voice: &str) -> Option<EspeakNg> {
    let dir = Path::new("espeak-ng-data");
    if !dir.join("phontab").exists() {
        eprintln!("[SKIP] no local espeak-ng-data");
        return None;
    }
    EspeakNg::builder().voice(voice).data_dir(dir).build().ok()
}

#[test]
fn two_words_get_ordered_word_events() {
    let Some(e) = engine("en") else { return };
    let (pcm, rate, events) = e.synth_with_events("hello world").expect("synth");
    assert_eq!(rate, 22050);

    let words: Vec<_> = events
        .iter()
        .filter(|ev| matches!(ev.kind, EventKind::Word(_)))
        .collect();
    assert_eq!(words.len(), 2, "expected one Word event per spoken word: {events:?}");

    // Word numbers are 1,2 and source positions count characters from 1:
    // "hello"@1, "world"@7 (C's transcript of this text).
    assert_eq!(words[0].kind, EventKind::Word(1));
    assert_eq!(words[1].kind, EventKind::Word(2));
    assert_eq!(words[0].text_position, 1);
    assert_eq!(words[1].text_position, 7);

    // Audio times: word 0 at/near the start, word 1 strictly later but before
    // the audio ends.
    assert!(words[0].audio_position_ms <= words[1].audio_position_ms);
    assert!(words[1].audio_position_ms > 0, "second word must start after the first");
    let total_ms = pcm.len() as u32 * 1000 / rate;
    assert!(words[1].audio_position_ms < total_ms, "word start must be within the audio");

    // Opens with the sample rate and the first sentence, closes with End and
    // MsgTerminated.
    assert_eq!(events.first().map(|e| &e.kind), Some(&EventKind::SampleRate(rate)));
    assert_eq!(events.get(1).map(|e| &e.kind), Some(&EventKind::Sentence(1)));
    assert_eq!(events.last().map(|e| &e.kind), Some(&EventKind::MsgTerminated));
    assert!(events.iter().any(|e| e.kind == EventKind::End(1)));
}

#[test]
fn single_word_gets_one_word_event() {
    let Some(e) = engine("en") else { return };
    let (_pcm, _rate, events) = e.synth_with_events("hello").expect("synth");
    let words = events.iter().filter(|ev| matches!(ev.kind, EventKind::Word(_))).count();
    assert_eq!(words, 1, "single word → one Word event: {events:?}");
    assert_eq!(
        events.iter().find(|e| matches!(e.kind, EventKind::Word(_))).unwrap().text_position,
        1
    );
}

#[test]
fn multi_clause_emits_multiple_sentence_events() {
    let Some(e) = engine("en") else { return };
    let (_pcm, _rate, events) = e.synth_with_events("Hello there. How are you?").expect("synth");

    let sentences: Vec<_> =
        events.iter().filter(|ev| matches!(ev.kind, EventKind::Sentence(_))).collect();
    assert!(sentences.len() >= 2, "two clauses → ≥2 Sentence events: {events:?}");
    assert_eq!(sentences[0].audio_position_ms, 0, "first sentence at the start");
    assert!(sentences[1].audio_position_ms > 0, "second sentence starts later");

    // The full stream (minus terminal markers) stays in audio order, with each
    // Sentence marker landing at or before its clause's first Word.
    let times: Vec<u32> = events
        .iter()
        .filter(|e| !matches!(e.kind, EventKind::End(_) | EventKind::MsgTerminated))
        .map(|e| e.audio_position_ms)
        .collect();
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "events out of audio order: {times:?}");
}

#[test]
fn word_audio_positions_are_monotonic() {
    let Some(e) = engine("en") else { return };
    let (_pcm, _rate, events) = e.synth_with_events("one two three four").expect("synth");
    let times: Vec<u32> = events
        .iter()
        .filter(|ev| matches!(ev.kind, EventKind::Word(_)))
        .map(|ev| ev.audio_position_ms)
        .collect();
    assert_eq!(times.len(), 4, "four words expected: {events:?}");
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "word times must not go backwards: {times:?}");
}
