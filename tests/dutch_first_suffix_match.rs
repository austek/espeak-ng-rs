// C's TranslateRules returns at the first standard ending it matches. The port
// kept scanning and used the last one, so `openen` took the `en` of the second
// syllable (stem `open`, doubled to `opeen`) instead of `enen` (stem `op`,
// doubled to `oop`). Expected mnemonics are from C espeak-ng master.

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
fn first_matching_ending_wins() {
    let cases = [("openen", "_!'o:p@n@n"), ("heropenen", "hEr_!'o:p@n@n")];
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no nl data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
