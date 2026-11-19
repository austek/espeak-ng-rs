// Long inputs must stay linear. Quadratic behaviour at 1 MB costs minutes, so
// the wall-clock bounds are far above any CI noise while still catching it.

use std::process::Command;
use std::time::{Duration, Instant};

fn run_fill(name: &str, unit: &str, bytes: usize, lang: &str) -> (Duration, Vec<u8>) {
    let path = std::env::temp_dir().join(format!("robustness_linear_{name}.txt"));
    let mut text = unit.repeat(bytes / unit.len() + 1);
    text.truncate(text.floor_char_boundary(bytes));
    std::fs::write(&path, text).expect("write input");
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let start = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "-f", path.to_str().expect("utf8")])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    let elapsed = start.elapsed();
    let _ = std::fs::remove_file(&path);
    assert!(out.status.success(), "{name}: {}", String::from_utf8_lossy(&out.stderr));
    (elapsed, out.stdout)
}

#[test]
fn one_megabyte_of_dots_takes_under_one_second() {
    let (elapsed, out) = run_fill("dots", ".", 1_000_000, "en");
    assert!(elapsed < Duration::from_secs(1), "took {elapsed:?}");
    assert_eq!(String::from_utf8_lossy(&out).trim(), "");
}

#[test]
fn one_megabyte_of_punctuation_and_short_words_is_linear() {
    for (name, unit) in [
        ("bang", "!"),
        ("query", "?"),
        ("hyphen", "-"),
        ("quote", "\""),
        ("abbrev", "a. "),
        ("cyrillic", "я. "),
    ] {
        let (elapsed, _) = run_fill(name, unit, 1_000_000, "en");
        assert!(elapsed < Duration::from_secs(20), "{name} took {elapsed:?}");
    }
}

/// Eight times the input costs eight times as much when linear and sixty-four times when
/// quadratic; a ratio holds in debug and release builds alike, an absolute bound does not.
#[test]
fn irish_capitals_inside_one_endless_word_are_linear() {
    let (small, _) = run_fill("camel_small", "aB", 200_000, "ga");
    let (large, _) = run_fill("camel_large", "aB", 1_600_000, "ga");
    assert!(large < small * 14, "200 KB took {small:?}, 1.6 MB took {large:?}");
}

#[test]
fn language_switched_words_load_their_data_once() {
    for (name, unit) in [("arabic", "مرحبا "), ("hindi", "नमस्ते "), ("korean", "한국어 ")] {
        let (elapsed, _) = run_fill(name, unit, 200_000, "en");
        assert!(elapsed < Duration::from_secs(20), "{name} took {elapsed:?}");
    }
}
