// C turns the apostrophe of a standalone 'n / 't into a schwa so the `ən` / `ət`
// dictionary entries read it. Expected mnemonics are from C espeak-ng master.

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

fn check(lang: &str, cases: &[(&str, &str)]) {
    for (text, want) in cases {
        let Some(got) = mnemonics(lang, text) else {
            eprintln!("[SKIP] no data");
            return;
        };
        assert_eq!(got, *want, "{lang} {text}");
    }
}

#[test]
fn dutch_article_apostrophes_read_as_schwa_words() {
    check(
        "nl",
        &[
            ("'n", "_|'@-n"),
            ("'t", "_|'@t"),
            ("\u{2019}t", "_|'@t"),
            ("'n huis", "_|@-n h'Wys"),
            ("ik zie 't huis", "_|Ik z'i _|@t h'Wys"),
            ("dat is 'n man", "dAt _|Is _|@-n m'An"),
            ("Wat is 't.", "v#At _|Is _|'@t"),
            ("Wat is 't\u{2026}", "v#At _|Is _|'@t"),
        ],
    );
}

#[test]
fn afrikaans_article_apostrophe_drops_the_n() {
    check("af", &[("Wat is 't.", "v'at _|Is _|@t"), ("'n, 't", "_|'@ _|'@t")]);
}
