// Long inputs must stay linear. Eight times the input costs eight times as much when linear and
// sixty-four times when quadratic, so each guard times an input of N and one of 8N and bounds
// their ratio: a ratio holds on a slow runner and in debug and release builds alike, where an
// absolute bound measures the machine.

use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Tests run on parallel threads; two long inputs at once starve each other and the clock
/// measures the neighbour, so a guard holds this lock for all of its runs.
static ONE_GUARD_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Allowed growth for eight times the input: 8x when linear, with room for noise and for the
/// cache effects of a larger heap, far below the 64x of quadratic code.
const GROWTH: u32 = 14;

/// Times below this are noise: a smaller input is judged as if it had taken this long.
const FLOOR: Duration = Duration::from_millis(50);

const SMALL: usize = 125_000;
const LARGE: usize = 1_000_000;

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

/// The fastest of three runs at `small` against one run at `large`, one after the other.
fn assert_linear_between(name: &str, unit: &str, lang: &str, small: usize, large: usize) -> Vec<u8> {
    // A debug build is about ten times slower, so it gets an eighth of the input; the ratio
    // still separates linear from quadratic growth.
    let (small, large) = if cfg!(debug_assertions) { (small / 8, large / 8) } else { (small, large) };
    let _alone = ONE_GUARD_AT_A_TIME.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let fastest = (0..3)
        .map(|_| run_fill(&format!("{name}_small"), unit, small, lang).0)
        .min()
        .expect("three runs");
    let (slow, out) = run_fill(&format!("{name}_large"), unit, large, lang);
    assert!(
        slow < fastest.max(FLOOR) * GROWTH,
        "{name}: {small} bytes took {fastest:?}, {large} bytes took {slow:?}"
    );
    out
}

fn assert_linear(name: &str, unit: &str, lang: &str) -> Vec<u8> {
    assert_linear_between(name, unit, lang, SMALL, LARGE)
}

#[test]
fn dots_are_linear() {
    let out = assert_linear("dots", ".", "en");
    assert_eq!(String::from_utf8_lossy(&out).trim(), "");
}

#[test]
fn punctuation_and_short_words_are_linear() {
    for (name, unit) in [
        ("bang", "!"),
        ("query", "?"),
        ("hyphen", "-"),
        ("quote", "\""),
        ("abbrev", "a. "),
        ("cyrillic", "я. "),
    ] {
        assert_linear(name, unit, "en");
    }
}

/// A clause end or a bracket after a clause end once cost a scan of everything before it.
#[test]
fn marks_that_end_or_open_a_clause_are_linear() {
    for (name, unit) in [
        ("query_blank", "? "),
        ("comma_blank", ", "),
        ("semicolon_blank", "; "),
        ("dot_blank", ". "),
        ("parens", "()"),
        ("angles", "<>"),
        ("parens_blank", "( )"),
        ("angles_blank", "< >"),
        ("ellipsis_blank", "\u{2026} "),
        ("em_dash_blank", "\u{2014} "),
        ("three_dots_blank", "... "),
        ("curly_quotes", "\u{201c}\u{201d}"),
    ] {
        assert_linear(name, unit, "en");
    }
}

#[test]
fn irish_capitals_inside_one_endless_word_are_linear() {
    assert_linear_between("camel", "aB", "ga", 200_000, 1_600_000);
}

/// A long clause with no word in it once cost a scan of the whole text per symbol, and a scan of
/// the whole clause per symbol after it.
#[test]
fn clauses_of_symbols_without_a_word_are_linear() {
    for (name, unit) in [("colon_slashes", "://"), ("colon_slash", ":/"), ("tildes", "~ ")] {
        assert_linear_between(name, unit, "en", 200_000, 1_600_000);
    }
}

#[test]
fn dotted_abbreviations_followed_by_a_letter_are_linear() {
    for (name, unit, lang) in [
        ("eg_at", "e.g. a@ ", "en"),
        ("eg_a", "e.g. a ", "en"),
        ("eg_ab", "e.g. ab ", "en"),
        ("ab_c", "a.b. c ", "en"),
        ("zb_a", "z.B. a ", "de"),
    ] {
        assert_linear_between(name, unit, lang, 20_000, 160_000);
    }
}

#[test]
fn language_switched_words_load_their_data_once() {
    for (name, unit) in [("arabic", "مرحبا "), ("hindi", "नमस्ते "), ("korean", "한국어 ")] {
        assert_linear_between(name, unit, "en", 25_000, 200_000);
    }
}
