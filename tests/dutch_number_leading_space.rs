// C writes a space before the word start that follows a clause's silent opening
// pause, so a vowel-initial Dutch number word has a leading space in --ipa
// while the same word typed out does not. Expected output is from C espeak-ng
// master.

use std::path::Path;
use std::process::Command;

fn ipa_raw(text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("nl_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "--ipa", "-v", "nl", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string())
}

#[test]
fn vowel_initial_number_words_keep_the_leading_space() {
    let cases = [
        ("1", " \u{2c8}e\u{2d0}n"),
        ("8", " \u{2c8}\u{251}xt"),
        ("11", " \u{2c8}\u{25b}lf"),
        ("een", "\u{2c8}e\u{2d0}n"),
        ("100", "h\u{2c8}\u{254}nd\u{259}rt"),
    ];
    for (text, want) in cases {
        let Some(got) = ipa_raw(text) else {
            eprintln!("[SKIP] no nl data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
