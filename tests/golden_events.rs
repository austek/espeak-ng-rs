// tests/golden_events.rs
//
// Event dumper for the golden WAV and event ratchet (tests/golden/wav_events.py).
// Reads one text per line from the file named by `GOLDEN_EV_FILE`, synthesises
// each with the voice `GOLDEN_EV_LANG` and prints, per row, a `=== <index>`
// marker followed by the transcript in the format of `probe.c evflat`
// (`ev <type> <pos> <len> <apos> <sample> <value>` lines, then `samples=<n>`).
// A row that fails to synthesise prints `ERR`.  `GOLDEN_EV_DATA` names the data
// directory.  Ignored by default: the python ratchet runs it.

use std::fmt::Write as _;
use std::path::Path;

use espeak_ng::{EspeakNg, EventKind};

fn line(kind: &EventKind) -> (u8, String) {
    match kind {
        EventKind::Word(n) => (1, format!("num={n}")),
        EventKind::Sentence(n) => (2, format!("num={n}")),
        EventKind::Mark(name) if name.is_empty() => (3, "name=(null)".into()),
        EventKind::Mark(name) => (3, format!("name={name}")),
        EventKind::Play(uri) => (4, format!("name={uri}")),
        EventKind::End(n) => (5, format!("num={n}")),
        EventKind::MsgTerminated => (6, "num=0".into()),
        EventKind::Phoneme(p) => (7, format!("ph=[{p}]")),
        EventKind::SampleRate(r) => (8, format!("num={r}")),
    }
}

fn transcript(engine: &EspeakNg, text: &str) -> Option<String> {
    let (pcm, _rate, events) = engine.synth_with_events(text).ok()?;
    let mut out = String::new();
    // The C probe runs synchronously, which never delivers MSG_TERMINATED.
    for ev in events.iter().filter(|e| !matches!(e.kind, EventKind::MsgTerminated)) {
        let (ty, value) = line(&ev.kind);
        let _ = writeln!(
            out,
            "ev {ty} {} {} {} {} {value}",
            ev.text_position, ev.length, ev.audio_position_ms, ev.sample
        );
    }
    let _ = writeln!(out, "samples={}", pcm.len());
    Some(out)
}

#[test]
#[ignore]
fn dump() {
    let file = std::env::var("GOLDEN_EV_FILE").expect("GOLDEN_EV_FILE");
    let lang = std::env::var("GOLDEN_EV_LANG").expect("GOLDEN_EV_LANG");
    let data = std::env::var("GOLDEN_EV_DATA").expect("GOLDEN_EV_DATA");
    let engine = EspeakNg::builder().voice(&lang).data_dir(Path::new(&data)).build().expect("engine");
    let rows = std::fs::read_to_string(file).expect("rows");
    let mut out = String::new();
    for (i, text) in rows.lines().enumerate() {
        let _ = writeln!(out, "=== {i}");
        out.push_str(&transcript(&engine, text).unwrap_or_else(|| "ERR\n".into()));
    }
    print!("{out}");
}
