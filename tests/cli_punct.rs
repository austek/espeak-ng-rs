// `--punct` announces clause punctuation as C does: a non-empty clause ends first, the name is a clause of its own, and a mark that ends the text is lost.

use std::process::Command;

fn spoken(args: &[&str]) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .env("ESPEAK_DATA_PATH", env!("CARGO_MANIFEST_DIR"))
        .arg("-q")
        .args(args)
        .output()
        .expect("spawn");
    assert_eq!(o.status.code(), Some(0));
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn bare_punct_names_every_clause_mark_on_its_own_line() {
    assert_eq!(spoken(&["--punct", "-x", "Hello, world."]), "h@l'oU\nk'0m@\nw'3:ld\n");
}

#[test]
fn ipa_output_breaks_the_same_way() {
    assert_eq!(spoken(&["--punct", "--ipa", "Hello, world."]), "həlˈəʊ\nkˈɒmə\nwˈɜːld\n");
}

#[test]
fn a_listed_mark_in_the_middle_of_the_text_is_announced() {
    let want = "h@l'oU\nk'0m@\nw'3:ld\nd'0t\nf'u:\n";
    assert_eq!(spoken(&["--punct=,.", "-x", "Hello, world. Foo"]), want);
}

#[test]
fn only_listed_marks_are_announced() {
    assert_eq!(spoken(&["--punct=,", "-x", "a, b"]), "'eI\nk'0m@\nb'i:\n");
    assert_eq!(spoken(&["--punct=", "-x", "hello, world."]), "h@l'oU\nw'3:ld\n");
}

#[test]
fn a_period_before_a_lower_case_word_still_ends_the_clause() {
    assert_eq!(spoken(&["--punct", "-x", "one. two"]), "w'0n\nd'0t\nt'u:\n");
}

#[test]
fn other_marks_use_their_own_names() {
    let want = "w'0n\n,Ekskl@m'eIS@n\nt'u:\n";
    assert_eq!(spoken(&["--punct", "-x", "one! two"]), want);
    assert_eq!(spoken(&["--punct", "-x", "one; two"]), "w'0n\ns,EmIk'oUl@n\nt'u:\n");
}

#[test]
/// Guards against regression; the behaviour already held before the punct fix.
fn without_the_option_marks_are_not_named() {
    assert_eq!(spoken(&["-x", "one, two"]), "w'0n\nt'u:\n");
}
