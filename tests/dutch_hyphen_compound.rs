// A hyphen joins two words without a word boundary in C's phoneme list: the
// first part keeps its final consonant voiced and the second loses its opening
// pause. Expected mnemonics are from C espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(lang: &str, text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("nl_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', " "))
}

#[test]
fn hyphen_compound_has_no_devoicing_or_pause_at_the_join() {
    let cases = [
        ("nl", "Zuid-Afrika", "z'Wyd'a:frik,a:"),
        ("nl", "Noord-Holland", "n'O:rdh'OlAnt"),
    ];
    for (lang, text, want) in cases {
        let Some(got) = mnemonics(lang, text) else {
            eprintln!("[SKIP] no data");
            return;
        };
        assert_eq!(got, want, "{lang} {text}");
    }
}
