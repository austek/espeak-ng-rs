// tests/async_events.rs
//
// Events in the asynchronous worker (the port's analogue of espeak's fifo mode):
// every utterance reports its events once, on its final chunk, in the order the
// synchronous engine gives; a cancelled utterance reports none; the next one is
// unaffected.  No test depends on timing: the worker is held at a rendezvous
// where a race would otherwise decide the outcome.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use espeak_ng::async_synth::AsyncSynth;
use espeak_ng::{EspeakNg, EventKind, SynthEvent};

fn have_data() -> bool {
    std::path::Path::new("espeak-ng-data/en_dict").exists()
}

/// Kind, text position and length: the event fields that do not depend on carried audio state.
type Shape = (EventKind, u32, u32);

fn shapes(events: &[SynthEvent]) -> Vec<Shape> {
    events.iter().map(|e| (e.kind.clone(), e.text_position, e.length)).collect()
}

/// The events of `text` from the synchronous engine, without the sample-rate event that only
/// the first call after a voice change carries.
fn sync_shapes(text: &str) -> Vec<Shape> {
    let engine = EspeakNg::new("en").expect("engine");
    without_rate(shapes(&engine.synth_with_events(text).expect("synth").2))
}

fn without_rate(events: Vec<Shape>) -> Vec<Shape> {
    events.into_iter().filter(|e| !matches!(e.0, EventKind::SampleRate(_))).collect()
}

type Finals = Vec<(String, Vec<Shape>)>;

#[derive(Default)]
struct Log {
    finals: Finals,
    audio_after_final: usize,
    early_events: usize,
    monotonic: bool,
}

#[test]
fn queued_utterances_each_report_their_events_once_in_order() {
    if !have_data() {
        eprintln!("[SKIP] no en data");
        return;
    }
    let texts = ["Hello world.", "How are you? Fine, thanks.", "One two three four."];
    let log = Arc::new(Mutex::new(Log { monotonic: true, ..Log::default() }));
    let sink = Arc::clone(&log);
    let synth = AsyncSynth::new("en", None, move |chunk| {
        let mut log = sink.lock().unwrap();
        if log.finals.iter().any(|(t, _)| t == chunk.text) && !chunk.is_final && !chunk.samples.is_empty() {
            log.audio_after_final += 1;
        }
        if chunk.is_final {
            let times: Vec<u32> = chunk
                .events
                .iter()
                .filter(|e| !matches!(e.kind, EventKind::MsgTerminated | EventKind::SampleRate(_)))
                .map(|e| e.audio_position_ms)
                .collect();
            log.monotonic &= times.windows(2).all(|w| w[0] <= w[1]);
            log.finals.push((chunk.text.to_string(), shapes(chunk.events)));
        } else if !chunk.events.is_empty() {
            log.early_events += 1;
        }
        false
    })
    .expect("engine");

    texts.iter().for_each(|t| synth.speak(*t));
    synth.synchronize();

    let log = log.lock().unwrap();
    let got: Vec<&str> = log.finals.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(got, texts, "one final chunk per utterance, in queue order");
    for (ix, (text, events)) in log.finals.iter().enumerate() {
        let has_rate = matches!(events.first().map(|e| &e.0), Some(EventKind::SampleRate(22050)));
        assert_eq!(has_rate, ix == 0, "only the first utterance reports the sample rate");
        assert_eq!(&without_rate(events.clone()), &sync_shapes(text), "events of {text:?}");
        assert_eq!(events.last().map(|e| &e.0), Some(&EventKind::MsgTerminated));
    }
    assert_eq!(log.early_events, 0, "events only come with the final chunk");
    assert_eq!(log.audio_after_final, 0);
    assert!(log.monotonic, "audio positions never go backwards within an utterance");
}

struct Gate {
    reached: Receiver<()>,
    release: Sender<()>,
}

#[test]
fn a_cancelled_utterance_reports_no_events_and_the_next_is_whole() {
    if !have_data() {
        eprintln!("[SKIP] no en data");
        return;
    }
    let long = "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs.";
    let next = "Hello world.";
    let (reached_tx, reached_rx) = channel::<()>();
    let (release_tx, release_rx) = channel::<()>();
    let gate = Gate { reached: reached_rx, release: release_tx };
    let finals: Arc<Mutex<Finals>> = Arc::default();
    let sink = Arc::clone(&finals);
    let mut gated = false;
    let synth = AsyncSynth::new("en", None, move |chunk| {
        if chunk.text == long && !gated {
            gated = true;
            reached_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        }
        if chunk.is_final {
            sink.lock().unwrap().push((chunk.text.to_string(), shapes(chunk.events)));
        }
        false
    })
    .expect("engine");

    synth.speak(long);
    gate.reached.recv().unwrap();
    synth.cancel();
    gate.release.send(()).unwrap();
    synth.speak(next);
    synth.synchronize();

    let finals = finals.lock().unwrap();
    assert!(finals.iter().all(|(t, _)| t != long), "the cancelled utterance reported events");
    assert_eq!(finals.len(), 1);
    assert_eq!(finals[0].0, next);
    assert_eq!(without_rate(finals[0].1.clone()), sync_shapes(next));
}
