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
    for s in scenarios() {
        let got = run_probe_with(probe_binary(), &s.args.iter().map(String::as_str).collect::<Vec<_>>());
        assert_eq!(got, fixture(s.fixture), "scenario {} {:?}", s.fixture, s.args);
    }
}

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
    for s in scenarios() {
        let text = run_scenario(&out, &s);
        std::fs::create_dir_all(fixture_path("expected")).expect("mkdir");
        std::fs::write(fixture_path(s.fixture), text).expect("write fixture");
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

#[test]
fn mark_events_reach_the_callback() {
    if !have_data() {
        eprintln!("[SKIP] no espeak-ng-data");
        return;
    }
    let out = run_probe(&["events", "2", "en", "0x1010", r#"<speak>Hi <mark name="m1"/> there.</speak>"#]);
    assert!(out.contains("type=3 ") && out.contains("name=m1"), "{out}");
}
