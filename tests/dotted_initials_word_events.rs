// A word that `CheckDottedAbbrev` joined from dotted initials and a one-letter word
// (`a.b. x` reads as `a.b.x`) is not in the clause text as such.  Its WORD event starts where the
// words of the text start, after the abbreviation in front of it.
// Expected values are the WORD events (text position, length) of C f0fb11a8's
// `espeak_Synth` with `espeakCHARS_UTF8 | espeakENDPAUSE`.

use std::path::Path;

use espeak_ng::{EspeakNg, EventKind};

/// (voice, text, `[(position, length)]` of the WORD events)
type Case = (&'static str, &'static str, &'static [(u32, u32)]);

const CASES: [Case; 4] = [
    ("en", "Mr. a.b. x", &[(1, 2), (4, 1)]),
    ("en", "Dr. u.s.a. x", &[(1, 2), (4, 1)]),
    ("de", "Hr. a.b. x", &[(1, 2), (4, 1)]),
    ("fr", "M. a.b. x", &[(1, 1), (3, 1)]),
];

#[test]
fn joined_initials_start_after_the_abbreviation() {
    let dir = Path::new("espeak-ng-data");
    if !dir.join("phontab").exists() {
        eprintln!("[SKIP] no local espeak-ng-data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .filter_map(|&(voice, text, want)| {
            let engine = EspeakNg::builder().voice(voice).data_dir(dir).build().ok()?;
            let (_, _, events) = engine.synth_with_events(text).expect("synth");
            let got: Vec<(u32, u32)> = events
                .iter()
                .filter(|e| matches!(e.kind, EventKind::Word(_)))
                .map(|e| (e.text_position, e.length))
                .collect();
            (got != want).then(|| format!("{voice} {text:?}: {got:?} (C {want:?})"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
