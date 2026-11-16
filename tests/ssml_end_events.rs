// tests/ssml_end_events.rs
//
// END, SAMPLERATE and PLAY events of markup against the transcripts the reference C library
// (f0fb11a8) prints (`tests/fixtures/c_events/`): one END per clause at the clause's
// `count_characters` with the sentence count, a SAMPLERATE at the start and after each voice
// change, and a PLAY one past the end of its clause.

use std::path::Path;

use espeak_ng::{EspeakNg, EventKind};

fn engine(accept_audio: bool) -> Option<EspeakNg> {
    let dir = Path::new("espeak-ng-data");
    if !dir.join("phontab").exists() {
        return None;
    }
    let mut e = EspeakNg::builder().voice("en").data_dir(dir).build().ok()?;
    e.set_markup(true);
    if accept_audio {
        e.set_uri_callback(Some(Box::new(|_, _| 0)));
    }
    Some(e)
}

/// `type pos len ms sample value` of the END, SAMPLERATE and PLAY events.
fn lines(text: &str, accept_audio: bool) -> Option<Vec<String>> {
    let (_, _, events) = engine(accept_audio)?.synth_with_events(text).ok()?;
    Some(
        events
            .iter()
            .filter_map(|e| {
                let (ty, value) = match &e.kind {
                    EventKind::End(n) => (5, format!("num={n}")),
                    EventKind::SampleRate(r) => (8, format!("num={r}")),
                    EventKind::Play(uri) => (4, format!("name={uri}")),
                    _ => return None,
                };
                Some(format!("{ty} {} {} {} {} {value}", e.text_position, e.length, e.audio_position_ms, e.sample))
            })
            .collect(),
    )
}

#[test]
fn plain_words_end_at_the_text_length() {
    let Some(got) = lines("hello world", false) else { return };
    assert_eq!(got, ["8 0 0 0 0 num=22050", "5 11 0 1028 22675 num=1"]);
}

#[test]
fn each_sentence_clause_ends_with_its_count() {
    let Some(got) = lines("<s>One</s><s>Two</s>", false) else { return };
    assert_eq!(
        got,
        [
            "8 0 0 0 0 num=22050",
            "5 2 0 526 11620 num=0",
            "5 9 0 1157 25514 num=1",
            "5 12 0 1383 30497 num=1",
            "5 19 0 2010 44330 num=2",
            "5 20 0 2017 44484 num=2",
        ]
    );
}

#[test]
fn accepted_audio_plays_one_past_the_clause_end() {
    let Some(got) = lines("one <audio src=\"x.wav\">alt</audio> two", true) else { return };
    assert_eq!(
        got,
        [
            "8 0 0 0 0 num=22050",
            "4 23 0 244 5387 name=x.wav",
            "5 22 0 244 5387 num=1",
            "5 33 0 244 5387 num=1",
            "5 38 0 872 19234 num=1",
        ]
    );
}

#[test]
fn a_voice_change_reports_the_sample_rate_again() {
    let Some(got) = lines("hello <voice xml:lang=\"de\">welt</voice> end", false) else { return };
    assert_eq!(
        got,
        [
            "8 0 0 0 0 num=22050",
            "5 26 0 359 7936 num=1",
            "8 0 0 359 7936 num=22050",
            "5 38 0 762 16816 num=1",
            "8 0 0 762 16816 num=22050",
            "5 43 0 1356 29900 num=1",
        ]
    );
}
