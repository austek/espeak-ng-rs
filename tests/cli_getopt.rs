// The command line is parsed as C's `getopt_long` does: abbreviations, permutation, error messages with exit status 0, atoi numerics, first positional only.

use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_espeak_cli");

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(BIN)
        .env("ESPEAK_DATA_PATH", env!("CARGO_MANIFEST_DIR"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("spawn");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn rejected(args: &[&str], message: &str) {
    assert_eq!(run(args), (Some(0), String::new(), format!("{BIN}: {message}\n")));
}

fn spoken(args: &[&str], phonemes: &str) {
    assert_eq!(run(args), (Some(0), phonemes.to_string(), String::new()));
}

#[test]
fn unknown_long_option() {
    rejected(&["--foo"], "unrecognized option '--foo'");
    rejected(&["--no-such=1"], "unrecognized option '--no-such=1'");
}

#[test]
fn unknown_short_option() {
    rejected(&["-Y"], "invalid option -- 'Y'");
}

#[test]
fn missing_arguments() {
    rejected(&["-a"], "option requires an argument -- 'a'");
    rejected(&["-v"], "option requires an argument -- 'v'");
    rejected(&["--path"], "option '--path' requires an argument");
}

#[test]
fn argument_on_a_flag_option() {
    rejected(&["--stdout=x"], "option '--stdout' doesn't allow an argument");
    rejected(&["--ver=3"], "option '--version' doesn't allow an argument");
}

#[test]
fn ambiguous_abbreviations_list_the_candidates() {
    rejected(
        &["--s"],
        "option '--s' is ambiguous; possibilities: '--stdin' '--stdout' '--split' '--sep' '--ssml-break'",
    );
    rejected(&["--st"], "option '--st' is ambiguous; possibilities: '--stdin' '--stdout'");
    rejected(
        &["--p"],
        "option '--p' is ambiguous; possibilities: '--punct' '--path' '--phonout' '--pho'",
    );
}

#[test]
fn unique_abbreviation_selects_the_option() {
    spoken(&["--i", "-q", "Hello world"], "həlˈəʊ wˈɜːld\n");
}

#[test]
/// Guards against regression; the behaviour already held before the getopt fix.
fn options_may_follow_the_text() {
    spoken(&["hi", "-q", "-x"], "h'aI\n");
}

#[test]
fn only_the_first_positional_is_spoken() {
    spoken(&["-q", "-x", "hi", "there"], "h'aI\n");
}

#[test]
fn numeric_arguments_use_atoi() {
    spoken(&["-q", "-x", "-s", "abc", "hi"], "h'aI\n");
    spoken(&["-q", "-x", "-b", "x", "hi"], "h'aI\n");
    spoken(&["-q", "-x", "-p", "200", "hi"], "h'aI\n");
}

#[test]
/// Guards against regression; the behaviour already held before the getopt fix.
fn clustered_short_options() {
    spoken(&["-qx", "hi"], "h'aI\n");
    spoken(&["-qvde", "-x", "hallo"], "h'alo:\n");
}
