// A clause with no stressed syllable puts the tonic on its last vowel, and C
// counts a non-syllabic vowel (`@-`) as a syllable for that. Expected
// mnemonics are from C espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("nl_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", "nl", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', " "))
}

fn check(cases: &[(&str, &str)]) {
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no nl data");
            return;
        };
        assert_eq!(got, *want, "{text}");
    }
}

#[test]
fn lone_nonsyllabic_schwa_takes_the_tonic() {
    check(&[("ən", "_|'@-n"), ("ət", "_|'@t")]);
}

#[test]
fn nonsyllabic_schwa_before_a_stressed_word_stays_light() {
    check(&[("ən man", "_|@-n m'An"), ("een man", "_|@n m'An")]);
}
