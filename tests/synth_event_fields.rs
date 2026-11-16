// tests/synth_event_fields.rs
//
// `SynthEvent` carries the fields of C's `espeak_EVENT` (`speak_lib.h`):
// 1-based character position, word length, sample position, and numbered
// `Sentence`/`End` events.  The first event reports the sample rate
// (`espeakEVENT_SAMPLERATE`, value 22050 in C's transcript of "hello world")
// and `MsgTerminated` closes the list with zeroed values, as C's retrieval
// mode delivers it (`event.c`, `espeakEVENT_MSG_TERMINATED`).

use std::path::Path;

use espeak_ng::{EspeakNg, EventKind};

fn engine() -> Option<EspeakNg> {
    let dir = Path::new("espeak-ng-data");
    if !dir.join("phontab").exists() {
        eprintln!("[SKIP] no local espeak-ng-data");
        return None;
    }
    EspeakNg::builder().voice("en").data_dir(dir).build().ok()
}

#[test]
fn kinds_carry_c_numbers() {
    assert_eq!(EventKind::Sentence(1), EventKind::Sentence(1));
    assert_eq!(EventKind::End(2), EventKind::End(2));
    assert_eq!(EventKind::SampleRate(22050), EventKind::SampleRate(22050));
    assert_ne!(EventKind::Play("a.wav".into()), EventKind::Mark("a.wav".into()));
}

#[test]
fn hello_world_opens_with_the_sample_rate() {
    let Some(e) = engine() else { return };
    let (_pcm, rate, events) = e.synth_with_events("hello world").expect("synth");
    assert_eq!(rate, 22050);
    let first = events.first().expect("events");
    assert_eq!(first.kind, EventKind::SampleRate(22050));
    assert_eq!((first.text_position, first.length, first.sample), (0, 0, 0));
}

#[test]
fn words_and_sentences_use_one_based_numbers_and_positions() {
    let Some(e) = engine() else { return };
    let (pcm, _, events) = e.synth_with_events("hello world").expect("synth");
    let kinds: Vec<_> = events.iter().map(|ev| ev.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            EventKind::SampleRate(22050),
            EventKind::Sentence(1),
            EventKind::Word(1),
            EventKind::Word(2),
            EventKind::End(1),
            EventKind::MsgTerminated,
        ]
    );
    // C: `ev 2 1 ...`, `ev 1 1 ...`, `ev 1 7 ...` (positions count characters from 1).
    let pos: Vec<u32> = events.iter().map(|ev| ev.text_position).collect();
    assert_eq!(pos, vec![0, 1, 1, 7, 11, 0]);
    // C's second word starts at sample 6776 of 22675; the sample field is the
    // position in samples, and the millisecond field follows from it.
    let second = &events[3];
    assert!(second.sample > 0 && (second.sample as usize) < pcm.len());
    assert_eq!(u64::from(second.audio_position_ms), second.sample * 1000 / 22050);
    let end = &events[4];
    assert_eq!(end.sample as usize, pcm.len());
}

#[test]
fn msg_terminated_is_zeroed_like_c() {
    let Some(e) = engine() else { return };
    let (_pcm, _, events) = e.synth_with_events("hello world").expect("synth");
    let last = events.last().expect("events");
    assert_eq!(last.kind, EventKind::MsgTerminated);
    assert_eq!((last.text_position, last.length, last.audio_position_ms, last.sample), (0, 0, 0, 0));
}

#[test]
fn sentences_are_numbered_in_order() {
    let Some(e) = engine() else { return };
    let (_pcm, _, events) = e.synth_with_events("One. Two.").expect("synth");
    let numbers: Vec<u32> = events
        .iter()
        .filter_map(|ev| match ev.kind {
            EventKind::Sentence(n) => Some(n),
            _ => None,
        })
        .collect();
    assert!(numbers.len() >= 2, "{events:?}");
    assert!(numbers.iter().enumerate().all(|(i, &n)| n == i as u32 + 1), "{numbers:?}");
}
