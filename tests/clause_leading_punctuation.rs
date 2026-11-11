// Punctuation before any letter or digit in a clause does not end it, so a
// leading `¿` or `¡` leaves no empty clause.  Expected mnemonics are from C
// espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(lang: &str, text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("en_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|"))
}

#[test]
fn inverted_marks_before_a_word_start_no_clause() {
    let cases = [
        ("es", "¿Cómo estás?", "k'omo est'as"),
        ("es", "¡Bien!", "bj'en"),
        ("es", "Hola. ¡Bien!", "'ola|bj'en"),
        ("es", "a. ¡Bien!", "'a|bj'en"),
        ("es", "¿Cómo estás? ¡Bien!", "k'omo est'as|bj'en"),
        ("es", "Hola, ¿qué tal?", "'ola|k'e t'al"),
    ];
    for (lang, text, want) in cases {
        let Some(got) = mnemonics(lang, text) else {
            eprintln!("[SKIP] no data");
            return;
        };
        assert_eq!(got, want, "{lang} {text}");
    }
}
