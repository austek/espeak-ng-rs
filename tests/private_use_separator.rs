// A private-use U+E000 in plain text separates words as C does; it is no clause boundary.

use std::process::Command;

fn spoken(flag: &str, text: &str) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .env("ESPEAK_DATA_PATH", env!("CARGO_MANIFEST_DIR"))
        .args(["-q", flag, text])
        .output()
        .expect("spawn");
    assert_eq!(o.status.code(), Some(0));
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn separates_two_words_without_a_clause_break() {
    assert_eq!(spoken("-x", "one\u{E000}two"), "w'0n t'u:\n");
    assert_eq!(spoken("--ipa", "one\u{E000}two"), "wˈɒn tˈuː\n");
}

#[test]
fn a_full_stop_before_it_is_read_as_dot() {
    assert_eq!(spoken("-x", "one.\u{E000}two"), "w'0n d'0t t'u:\n");
    assert_eq!(spoken("--ipa", "one.\u{E000}two"), "wˈɒn dˈɒt tˈuː\n");
}

#[test]
fn edges_and_blanks_add_nothing() {
    assert_eq!(spoken("-x", "\u{E000}one"), "w'0n\n");
    assert_eq!(spoken("-x", "one \u{E000} two"), "w'0n t'u:\n");
}
