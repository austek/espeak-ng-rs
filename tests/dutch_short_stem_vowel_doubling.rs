// Taking `en` off a two-letter stem still restores the doubled vowel: C sees
// the space before the word as a non-vowel, so `open` → `oop` reads long.
// Expected mnemonics are from C espeak-ng master.

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

#[test]
fn two_letter_stems_double_their_vowel() {
    let cases = [("open", "_!'o:p@n"), ("opent", "_!'o:p@nt"), ("graden", "Qr'a:d@n")];
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no nl data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
