//! The C probe program, linked against this crate's `libespeak-ng.so`, must
//! print exactly what it prints against the reference C library.  The
//! expectations in `tests/fixtures/expected/` were recorded from that C
//! library by `record_fixtures_from_c` (see below).
#![cfg(target_os = "linux")]

mod common;

use common::*;
use std::path::{Path, PathBuf};

fn cases_path() -> String {
    fixture_path("phoneme_cases.tsv").to_string_lossy().into_owned()
}

struct Scenario {
    fixture: &'static str,
    args: Vec<String>,
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}

const WAV_CASES: &[(&str, &str)] = &[
    ("en", "Hello world, this is a test."),
    ("de", "Guten Morgen, wie geht es dir?"),
    ("fr", "Bonjour tout le monde."),
    ("ru", "Привет, мир! Это тест."),
    ("ku", "Roj baş, ev ceribandin e."),
    ("ar", "مرحبا بالعالم"),
    ("tr", "Merhaba dünya, bu bir deneme."),
    ("en", "Numbers 1 2 3 and 4.5 percent"),
];

/// Flags `0` leave out `espeakENDPAUSE`: the synthesis ends without the final pause.
fn final_pause_scenarios() -> Vec<Scenario> {
    let text = "Hello world, this is a test.";
    vec![
        Scenario { fixture: "expected/wav_no_final_pause.txt", args: args(&["wav", "en", "0x0", text]) },
        Scenario {
            fixture: "expected/events_no_final_pause.txt",
            args: args(&["events", "1", "en", "0x0", text]),
        },
    ]
}

/// A closing quote after the last mark is a clause of its own.
fn closing_quote_scenarios() -> Vec<Scenario> {
    [("en", "Come here!\""), ("en", "Hello! \""), ("de", "Komm her.\"")]
        .iter()
        .enumerate()
        .map(|(i, (lang, text))| {
            let fixture: &'static str = Box::leak(format!("expected/closing_quote_{i}.txt").into_boxed_str());
            Scenario { fixture, args: args(&["phon", lang, "2", text]) }
        })
        .collect()
}

const MARK_TEXT: &str = r#"One <mark name="a"/> two. Three, four."#;
const AUDIO_TEXT: &str = r#"Hi <audio src="a.wav">there</audio> you."#;
/// The URI callback runs while the reader is in the clause that holds the tag, after the
/// buffers of the clauses before it.
const URI_TEXT: &str = r#"<speak>Hi <audio src="a.wav">there</audio>. Next sentence here is long enough. <audio src="b.wav"/> end.</speak>"#;
const VOICE_TEXT: &str = r#"<speak>Hi <voice name="de">Hallo Welt.</voice> bye.</speak>"#;

/// One scenario per event kind and delivery path, recorded from C.  The last
/// `events` argument is the probe's option word: `espeak_Initialize` options,
/// plus 0x100 to use `espeak_ng_SetPhonemeEvents` and 0x200 for a URI
/// callback that refuses.
fn event_scenarios() -> Vec<Scenario> {
    let rows: [(&str, &str, &str, &str, &str, &str); 14] = [
        ("uri_timing_mode1", "1", "en", "0x1010", URI_TEXT, "0"),
        ("uri_timing_mode2", "2", "en", "0x1010", URI_TEXT, "0"),
        ("uri_timing_refused", "1", "en", "0x1010", URI_TEXT, "512"),
        ("mark_mode1", "1", "en", "0x1010", MARK_TEXT, "0"),
        ("mark_mode2", "2", "en", "0x1010", MARK_TEXT, "0"),
        ("play_accepted", "1", "en", "0x1010", AUDIO_TEXT, "0"),
        ("play_refused", "1", "en", "0x1010", AUDIO_TEXT, "512"),
        ("voice_mode1", "1", "en", "0x1010", VOICE_TEXT, "0"),
        ("voice_mode2", "2", "en", "0x1010", VOICE_TEXT, "0"),
        ("phoneme_mode1", "1", "en", "0x1000", "Hi there.", "1"),
        ("phoneme_mode2", "2", "en", "0x1000", "Hi there.", "1"),
        ("phoneme_ipa_de", "2", "de", "0x1000", "Guten Tag.", "3"),
        ("phoneme_ipa_only_option", "2", "en", "0x1000", "Hi.", "2"),
        ("phoneme_ng_ipa_mode1", "1", "en", "0x1000", "Hi there.", "259"),
    ];
    rows.iter()
        .map(|(name, mode, voice, flags, text, options)| {
            let fixture: &'static str = Box::leak(format!("expected/events_{name}.txt").into_boxed_str());
            Scenario { fixture, args: args(&["events", mode, voice, flags, text, options]) }
        })
        .collect()
}

/// `espeak_SetPhonemeTrace` and `espeak_SetPhonemeCallback`: one phoneme line per clause.
fn trace_scenarios() -> Vec<Scenario> {
    let rows: [(&str, &str, &str, &str, &str); 3] = [
        ("mnemonics", "1", "en", "Hello world, this is a test. Second one.", "0"),
        ("ipa_de_callback", "2", "de", "Guten Tag. Wie geht es dir?", "1"),
        ("callback_only", "0", "en", "Hi. Bye.", "1"),
    ];
    let mut list: Vec<Scenario> = rows
        .iter()
        .map(|(name, mode, voice, text, callback)| {
            let fixture: &'static str = Box::leak(format!("expected/trace_{name}.txt").into_boxed_str());
            Scenario { fixture, args: args(&["trace", mode, voice, "0x1000", text, callback]) }
        })
        .collect();
    list.extend(ssml_trace_scenarios());
    list
}

/// Trace bit 3 also prints C's word-lookup lines; only the `Replace:` ones are ported, so these
/// scenarios are recorded in full and compared on those lines alone.
fn replace_trace_scenarios() -> Vec<Scenario> {
    let rows: [(&str, &str, &str); 3] = [
        ("en", "en", "Tom & Jerry cost 5 \u{20ac} today \u{1f600} ok."),
        ("de", "de", "Das ist \u{2211} und \u{c4}pfel \u{a7} 3 \u{b0} C. Zweiter Satz mit \u{20ac}."),
        ("cmn", "cmn", "\u{4f60}\u{597d}\u{1f600}"),
    ];
    rows.iter()
        .map(|(name, voice, text)| {
            let fixture: &'static str = Box::leak(format!("expected/trace_replace_{name}.txt").into_boxed_str());
            Scenario { fixture, args: args(&["trace", "9", voice, "0x1000", text, "0"]) }
        })
        .collect()
}

fn replace_lines(output: &str) -> Vec<&str> {
    output.lines().filter(|l| l.starts_with("Replace: ")).collect()
}

fn assert_replace_lines(fixture_name: &str) {
    if !have_data() {
        eprintln!("[SKIP] no espeak-ng-data");
        return;
    }
    let s = replace_trace_scenarios().into_iter().find(|s| s.fixture == fixture_name).expect("scenario");
    let got = run_probe_with(probe_binary(), &s.args.iter().map(String::as_str).collect::<Vec<_>>());
    let want = fixture(s.fixture);
    assert!(!replace_lines(&want).is_empty(), "{fixture_name} has no Replace line");
    assert_eq!(replace_lines(&got), replace_lines(&want), "{fixture_name}");
}

#[test]
fn trace_bit3_prints_the_replace_lines_of_listed_replacement_text() {
    assert_replace_lines("expected/trace_replace_en.txt");
}

#[test]
fn trace_bit3_prints_the_replace_lines_before_each_clause() {
    assert_replace_lines("expected/trace_replace_de.txt");
}

#[test]
fn trace_bit3_prints_the_replace_lines_of_a_multi_word_entry() {
    assert_replace_lines("expected/trace_replace_cmn.txt");
}

const SSML_TRACE_TEXT: &str = "<speak>Hi there.</speak>";
const SSML_TRACE_CLAUSES: &str = r#"<speak>Hi <voice name="de">Hallo Welt.</voice> bye. <break time="200ms"/> Done now.</speak>"#;

/// The trace of markup: one line per clause, empty clauses included, in every mode.
fn ssml_trace_scenarios() -> Vec<Scenario> {
    let rows: [(&str, &str, &str, &str, &str, &str); 8] = [
        ("ssml_mode0", "0", "en", "0x1010", SSML_TRACE_TEXT, "0"),
        ("ssml_mode1", "1", "en", "0x1010", SSML_TRACE_TEXT, "0"),
        ("ssml_mode1_callback", "1", "en", "0x1010", SSML_TRACE_TEXT, "1"),
        ("ssml_mode2", "2", "en", "0x1010", SSML_TRACE_TEXT, "0"),
        ("ssml_mode3_callback", "3", "en", "0x1010", SSML_TRACE_TEXT, "1"),
        ("ssml_callback_only", "0", "en", "0x1010", SSML_TRACE_TEXT, "1"),
        ("ssml_clauses_mode1", "1", "en", "0x1010", SSML_TRACE_CLAUSES, "1"),
        ("ssml_clauses_ipa", "2", "en", "0x1010", SSML_TRACE_CLAUSES, "0"),
    ];
    rows.iter()
        .map(|(name, mode, voice, flags, text, callback)| {
            let fixture: &'static str = Box::leak(format!("expected/trace_{name}.txt").into_boxed_str());
            Scenario { fixture, args: args(&["trace", mode, voice, flags, text, callback]) }
        })
        .collect()
}

fn scenarios() -> Vec<Scenario> {
    let mut list = vec![
        Scenario { fixture: "expected/batch.txt", args: vec!["batch".into(), cases_path()] },
        Scenario { fixture: "expected/voices.txt", args: args(&["voices"]) },
        Scenario { fixture: "expected/params.txt", args: args(&["params"]) },
        Scenario { fixture: "expected/ngapi.txt", args: args(&["ngapi"]) },
        Scenario { fixture: "expected/events_retrieval.txt", args: args(&["events", "1", "en", "0x1000", "Hi."]) },
        Scenario { fixture: "expected/events_sync_de.txt", args: args(&["events", "2", "de", "0x1000", "Guten Tag."]) },
    ];
    for (i, (lang, text)) in WAV_CASES.iter().enumerate() {
        let fixture: &'static str = Box::leak(format!("expected/wav_{i}.txt").into_boxed_str());
        list.push(Scenario { fixture, args: args(&["wav", lang, "0x1000", text]) });
    }
    list.extend(final_pause_scenarios());
    list.extend(closing_quote_scenarios());
    list.extend(event_scenarios());
    list.extend(trace_scenarios());
    list
}

fn run_scenario(binary: &Path, s: &Scenario) -> String {
    let refs: Vec<&str> = s.args.iter().map(String::as_str).collect();
    run_probe_with(binary, &refs)
}

#[test]
fn probe_output_matches_the_c_library() {
    if !have_data() {
        eprintln!("[SKIP] no espeak-ng-data");
        return;
    }
    let mismatched: Vec<&str> = scenarios()
        .iter()
        .filter(|s| run_probe_with(probe_binary(), &s.args.iter().map(String::as_str).collect::<Vec<_>>()) != fixture(s.fixture))
        .map(|s| s.fixture)
        .collect();
    assert!(mismatched.is_empty(), "scenarios differ from the C recording: {mismatched:?}");
}

/// The scenario whose fixture is `fixture`, compared on its own so a failure names the diff.
fn assert_scenario(fixture_name: &str) {
    if !have_data() {
        eprintln!("[SKIP] no espeak-ng-data");
        return;
    }
    let s = scenarios().into_iter().find(|s| s.fixture == fixture_name).expect("scenario");
    let got = run_probe_with(probe_binary(), &s.args.iter().map(String::as_str).collect::<Vec<_>>());
    assert_eq!(got, fixture(s.fixture), "{fixture_name}");
}

macro_rules! event_kind_tests {
    ($($test:ident => $fixture:literal),* $(,)?) => {
        $(#[test] fn $test() { assert_scenario($fixture); })*
    };
}

event_kind_tests! {
    mark_event_in_retrieval_mode => "expected/events_mark_mode1.txt",
    mark_event_in_synchronous_mode => "expected/events_mark_mode2.txt",
    play_event_when_the_uri_callback_accepts => "expected/events_play_accepted.txt",
    play_event_absent_when_the_uri_callback_refuses => "expected/events_play_refused.txt",
    voice_change_sample_rate_event_in_retrieval_mode => "expected/events_voice_mode1.txt",
    voice_change_sample_rate_event_in_synchronous_mode => "expected/events_voice_mode2.txt",
    phoneme_events_from_initialize_options_in_retrieval_mode => "expected/events_phoneme_mode1.txt",
    phoneme_events_from_initialize_options_in_synchronous_mode => "expected/events_phoneme_mode2.txt",
    phoneme_events_name_phonemes_in_ipa_for_german => "expected/events_phoneme_ipa_de.txt",
    ipa_initialize_option_alone_enables_phoneme_events => "expected/events_phoneme_ipa_only_option.txt",
    phoneme_events_from_set_phoneme_events => "expected/events_phoneme_ng_ipa_mode1.txt",
    phoneme_trace_prints_one_line_per_clause => "expected/trace_mnemonics.txt",
    phoneme_trace_in_ipa_with_the_phoneme_callback => "expected/trace_ipa_de_callback.txt",
    phoneme_callback_without_a_trace => "expected/trace_callback_only.txt",
    uri_callback_runs_with_its_clause_in_retrieval_mode => "expected/events_uri_timing_mode1.txt",
    uri_callback_runs_with_its_clause_in_synchronous_mode => "expected/events_uri_timing_mode2.txt",
    uri_callback_runs_with_its_clause_when_it_refuses => "expected/events_uri_timing_refused.txt",
    ssml_trace_mode0 => "expected/trace_ssml_mode0.txt",
    ssml_trace_mode1 => "expected/trace_ssml_mode1.txt",
    ssml_trace_mode1_callback => "expected/trace_ssml_mode1_callback.txt",
    ssml_trace_mode2 => "expected/trace_ssml_mode2.txt",
    ssml_trace_mode3_callback => "expected/trace_ssml_mode3_callback.txt",
    ssml_phoneme_callback_without_a_trace => "expected/trace_ssml_callback_only.txt",
    ssml_trace_one_line_per_clause => "expected/trace_ssml_clauses_mode1.txt",
    ssml_trace_in_ipa_per_clause => "expected/trace_ssml_clauses_ipa.txt",
}

/// Fixtures the recorder leaves alone: they predate the probe's current form.
const KEEP_RECORDED: &[&str] = &[
    "expected/events_retrieval.txt",
    "expected/events_no_final_pause.txt",
    "expected/events_sync_de.txt",
];

/// Records the expectations from the reference C library.  Needs
/// `CAPI_C_BUILD` (the C build directory) and `CAPI_C_INCLUDE` (its include
/// directory); run with `--ignored`.
#[test]
#[ignore]
fn record_fixtures_from_c() {
    let build = PathBuf::from(std::env::var("CAPI_C_BUILD").expect("CAPI_C_BUILD"));
    let include = std::env::var("CAPI_C_INCLUDE").expect("CAPI_C_INCLUDE");
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("probe_c");
    let libs = [
        build.join("src/libespeak-ng/libespeak-ng.a"),
        build.join("src/speechPlayer/libspeechPlayer.a"),
        build.join("src/ucd-tools/libucd.a"),
    ];
    let libs: Vec<String> = libs.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    let mut extra: Vec<&str> = vec!["-I", &include];
    extra.extend(libs.iter().map(String::as_str));
    extra.extend(["-lm", "-lpthread", "-lstdc++"]);
    compile_c(&manifest_dir().join("tests/c/probe.c"), &out, &extra);
    let recorded = scenarios().into_iter().chain(replace_trace_scenarios());
    for s in recorded.filter(|s| !KEEP_RECORDED.contains(&s.fixture)) {
        let text = run_scenario(&out, &s);
        std::fs::create_dir_all(fixture_path("expected")).expect("mkdir");
        std::fs::write(fixture_path(s.fixture), text).expect("write fixture");
    }
    record_event_transcripts(&out);
}

/// Writes `tests/fixtures/c_events/<id>.txt` in the workspace root for every
/// row of `cases.tsv` (id, options, voice, flags, text).
fn record_event_transcripts(binary: &Path) {
    let dir = manifest_dir().join("..").join("tests").join("fixtures").join("c_events");
    let cases = std::fs::read_to_string(dir.join("cases.tsv")).expect("cases.tsv");
    for line in cases.lines().filter(|l| !l.is_empty()) {
        let f: Vec<&str> = line.splitn(5, '\t').collect();
        assert_eq!(f.len(), 5, "bad case row: {line}");
        let text = run_probe_with(binary, &["evflat", f[1], f[2], f[3], f[4]]);
        std::fs::write(dir.join(format!("{}.txt", f[0])), text).expect("write transcript");
    }
}

#[test]
fn wav_samples_equal_the_engine_output() {
    if !have_data() {
        eprintln!("[SKIP] no espeak-ng-data");
        return;
    }
    let dir = espeak_ng::translate::data_dir_from_path(&data_root()).expect("data dir");
    for (lang, text) in WAV_CASES {
        let engine = espeak_ng::EspeakNg::with_data_dir(lang, &dir).expect("engine");
        let (pcm, rate) = engine.synth(text).expect("synth");
        let line = run_probe(&["wav", lang, "0x1000", text]);
        let want = format!("rate={rate} synth=0 samples={} fnv={:016x}\n", pcm.len(), fnv_samples(&pcm));
        assert_eq!(line, want, "{lang}: {text}");
    }
}

#[test]
fn invalid_calls_return_error_codes() {
    if !have_data() {
        eprintln!("[SKIP] no espeak-ng-data");
        return;
    }
    let got = run_probe(&["errors"]);
    let want = "synth-before-init=-1\nsynth-null=-1\nsynth-empty=0\nbyname-null=2\n\
                props-null=2\npunct-null=0\nbad-param=-1\nbad-get=0\nterminate=0\nafter-terminate=-1\n";
    assert_eq!(got, want);
}

#[test]
fn multi_clause_events_keep_the_c_structure() {
    if !have_data() {
        eprintln!("[SKIP] no espeak-ng-data");
        return;
    }
    let text = "Hello world, this is a test. Second one.";
    let out = run_probe(&["events", "2", "en", "0x1000", text]);
    let types: Vec<&str> = out.lines().filter(|l| l.starts_with("  ev ")).collect();
    let count = |t: &str| types.iter().filter(|l| l.contains(&format!("type={t} "))).count();
    assert_eq!(count("8"), 1, "one sample-rate event");
    assert_eq!(count("2"), 2, "two sentences");
    assert_eq!(count("1"), 8, "eight words");
    assert_eq!(count("5"), 3, "three clause ends");
    assert!(out.contains("num=7"), "words are numbered across the message");
    assert!(out.lines().rev().nth(1).is_some_and(|l| l.contains("wav=null")));
}

#[test]
fn python_ctypes_round_trip() {
    if !have_data() || std::process::Command::new("python3").arg("--version").output().is_err() {
        eprintln!("[SKIP] no data or python3");
        return;
    }
    let lib = dist_dir().join("libespeak-ng.so");
    let out = std::process::Command::new("python3")
        .arg("-I")
        .arg(manifest_dir().join("tests/py/ctypes_check.py"))
        .arg(&lib)
        .arg(manifest_dir().join("tests/fixtures"))
        .env("ESPEAK_DATA_PATH", data_root())
        .output()
        .expect("run python3");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success() && text.contains("PASS"), "{text}\n{}", String::from_utf8_lossy(&out.stderr));
}

/// Plain text has no name table: C delivers `id.name == NULL` for a `\x01NM` mark, which the
/// probe prints as `(null)` and tells apart from an empty name.
#[test]
fn plain_text_marks_have_a_null_name() {
    if !have_data() {
        eprintln!("[SKIP] no espeak-ng-data");
        return;
    }
    let dir = manifest_dir().join("..").join("tests").join("fixtures").join("c_events");
    let cases = std::fs::read_to_string(dir.join("cases.tsv")).expect("cases.tsv");
    let rows: Vec<Vec<&str>> = cases
        .lines()
        .filter(|l| l.starts_with("plain_mark"))
        .map(|l| l.splitn(5, '\t').collect())
        .collect();
    assert_eq!(rows.len(), 4, "the four plain mark cases");
    for f in rows {
        let got = run_probe(&["evflat", f[1], f[2], f[3], f[4]]);
        let want = std::fs::read_to_string(dir.join(format!("{}.txt", f[0]))).expect("transcript");
        assert!(want.contains("name=(null)"), "{}: the recording has a NULL name", f[0]);
        assert_eq!(got, want, "{}", f[0]);
    }
}

#[test]
fn mark_events_reach_the_callback() {
    if !have_data() {
        eprintln!("[SKIP] no espeak-ng-data");
        return;
    }
    let out = run_probe(&["events", "2", "en", "0x1010", r#"<speak>Hi <mark name="m1"/> there.</speak>"#]);
    assert!(out.contains("type=3 ") && out.contains("name=m1"), "{out}");
}
