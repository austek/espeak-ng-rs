// tests/c_events.rs
//
// Event ratchet.  `tests/fixtures/c_events/<id>.txt` holds the event
// transcript the reference C library (f0fb11a8) prints for each row of
// `cases.tsv`, recorded by `record_fixtures_from_c` in `capi/tests/c_abi.rs`
// through the probe's `evflat` scenario.
//
// Each line is `ev <type> <pos> <len> <apos> <sample> <value>`, followed by
// `samples=<total>`.  `known_diff.txt` lists the cases where the Rust
// transcript still differs, as `<id><TAB><reason>`: `audio` when the sample
// total differs from C, `events` when the audio matches and only the events
// differ.  A case outside the list must match exactly; a listed case that now
// matches must be removed, so the list only shrinks.

use std::collections::BTreeMap;
use std::path::Path;

use espeak_ng::{EspeakNg, EventKind};

const SSML_FLAG: u32 = 0x10;
const PHONEME_EVENTS: u32 = 0x1;
const PHONEME_IPA: u32 = 0x2;
/// The probe installs a URI callback that accepts `<audio>` URIs.
const ACCEPT_URI: u32 = 0x10000;

struct Case {
    id: String,
    options: u32,
    voice: String,
    flags: u32,
    text: String,
}

fn fixtures() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/c_events")
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('x') => {
                let hex: String = it.by_ref().take(2).collect();
                out.push(char::from(u8::from_str_radix(&hex, 16).expect("hex escape")));
            }
            Some('u') => {
                let hex: String = it.by_ref().skip(1).take_while(|&h| h != '}').collect();
                out.push(char::from_u32(u32::from_str_radix(&hex, 16).expect("hex")).expect("scalar"));
            }
            other => panic!("bad escape {other:?} in {s:?}"),
        }
    }
    out
}

fn cases() -> Vec<Case> {
    let tsv = std::fs::read_to_string(fixtures().join("cases.tsv")).expect("cases.tsv");
    tsv.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let f: Vec<&str> = l.splitn(5, '\t').collect();
            assert_eq!(f.len(), 5, "bad case row: {l}");
            Case {
                id: f[0].to_string(),
                options: f[1].parse().expect("options"),
                voice: f[2].to_string(),
                flags: u32::from_str_radix(f[3].trim_start_matches("0x"), 16).expect("flags"),
                text: unescape(f[4]),
            }
        })
        .collect()
}

fn known_diff() -> BTreeMap<String, String> {
    let path = fixtures().join("known_diff.txt");
    let text = std::fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let (id, reason) = l.split_once('\t').expect("id<TAB>reason");
            (id.to_string(), reason.to_string())
        })
        .collect()
}

fn engine(case: &Case) -> Option<EspeakNg> {
    let dir = Path::new("espeak-ng-data");
    if !dir.join("phontab").exists() {
        return None;
    }
    let mut e = EspeakNg::builder().voice(&case.voice).data_dir(dir).build().ok()?;
    e.set_markup(case.flags & SSML_FLAG != 0);
    e.set_phoneme_events(case.options & PHONEME_EVENTS != 0, case.options & PHONEME_IPA != 0);
    if case.options & ACCEPT_URI != 0 {
        e.set_uri_callback(Some(Box::new(|_, _| 0)));
    }
    Some(e)
}

fn line(kind: &EventKind) -> (u8, String) {
    match kind {
        EventKind::Word(n) => (1, format!("num={n}")),
        EventKind::Sentence(n) => (2, format!("num={n}")),
        // The engine reports no name for a plain-text mark; the C API turns that into NULL
        // (`plain_text_marks_have_a_null_name` in capi/tests/c_abi.rs checks it).
        EventKind::Mark(name) if name.is_empty() => (3, "name=(null)".into()),
        EventKind::Mark(name) => (3, format!("name={name}")),
        EventKind::Play(uri) => (4, format!("name={uri}")),
        EventKind::End(n) => (5, format!("num={n}")),
        EventKind::MsgTerminated => (6, "num=0".into()),
        EventKind::Phoneme(p) => (7, format!("ph=[{p}]")),
        EventKind::SampleRate(r) => (8, format!("num={r}")),
    }
}

/// The Rust transcript in the C line format.
fn transcript(case: &Case) -> Option<String> {
    let e = engine(case)?;
    let (pcm, _rate, events) = e.synth_with_events(&case.text).ok()?;
    let mut out = String::new();
    // The probe runs C in synchronous mode, which never delivers MSG_TERMINATED.
    for ev in events.iter().filter(|e| !matches!(e.kind, EventKind::MsgTerminated)) {
        let (ty, value) = line(&ev.kind);
        out.push_str(&format!(
            "ev {ty} {} {} {} {} {value}\n",
            ev.text_position, ev.length, ev.audio_position_ms, ev.sample
        ));
    }
    out.push_str(&format!("samples={}\n", pcm.len()));
    Some(out)
}

fn recorded(case: &Case) -> String {
    std::fs::read_to_string(fixtures().join(format!("{}.txt", case.id)))
        .unwrap_or_else(|e| panic!("transcript for {}: {e}", case.id))
}

fn total(transcript: &str) -> &str {
    transcript.lines().last().unwrap_or_default()
}

fn reason(c: &Case, got: &str) -> &'static str {
    if total(got) == total(&recorded(c)) {
        "events"
    } else {
        "audio"
    }
}

#[test]
fn transcripts_match_c_except_the_known_diff() {
    let known = known_diff();
    let mut failures = Vec::new();
    for c in cases() {
        let Some(got) = transcript(&c) else { return };
        let same = got == recorded(&c);
        match (known.get(&c.id), same) {
            (None, true) => {}
            (None, false) => failures.push(format!("{}: differs from C and is not in known_diff", c.id)),
            (Some(_), true) => failures.push(format!("{}: now matches C, remove it from known_diff", c.id)),
            (Some(r), false) if r != reason(&c, &got) => {
                failures.push(format!("{}: known_diff says {r}, actual {}", c.id, reason(&c, &got)));
            }
            (Some(_), false) => {}
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn known_diff_names_only_recorded_cases() {
    let ids: Vec<String> = cases().into_iter().map(|c| c.id).collect();
    for id in known_diff().keys() {
        assert!(ids.contains(id), "known_diff lists unknown case {id}");
    }
}

/// Prints the Rust and C transcripts of the case named by `C_EVENTS_CASE`.
#[test]
#[ignore]
fn print_case() {
    let id = std::env::var("C_EVENTS_CASE").expect("C_EVENTS_CASE");
    let c = cases().into_iter().find(|c| c.id == id).expect("case");
    println!("rust:\n{}", transcript(&c).unwrap_or_default());
    println!("c:\n{}", recorded(&c));
}

/// Prints the `known_diff.txt` rows for the current Rust output.
#[test]
#[ignore]
fn print_known_diff() {
    for c in cases() {
        let Some(got) = transcript(&c) else { return };
        if got != recorded(&c) {
            println!("{}\t{}", c.id, reason(&c, &got));
        }
    }
}
