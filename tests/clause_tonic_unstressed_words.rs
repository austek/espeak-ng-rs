// A clause made only of unstressed words puts its tonic on the last word.
// Expected mnemonics are from C espeak-ng master.

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
fn the_last_unstressed_word_takes_the_tonic() {
    let cases = [
        ("it it", "It 'It"),
        ("is the", "Iz D'@"),
        ("St is", "s@nt 'Iz"),
        ("the St", "D@ s'@nt"),
        ("it it it it", "It It It 'It"),
    ];
    for (text, want) in cases {
        let Some(got) = mnemonics("en", text) else {
            eprintln!("[SKIP] no data");
            return;
        };
        assert_eq!(got, want, "en {text}");
    }
}
