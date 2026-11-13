// The `yue-Latn-jyutping` voice file sets `dictrules 2`, which selects the
// jyutping letter rules for Latin text. Expected mnemonics are from C
// espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(lang: &str, text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("yue_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', "/"))
}

#[test]
fn jyutping_voice_reads_latin_as_jyutping() {
    let cases = [
        ("gin3", "g'i3n_|"),
        ("maan5", "m'aa5n_|"),
        ("baat3", "b'aa3t_|"),
        ("ngo5 oi3", "N'o5_| 'oi3_|"),
    ];
    for (text, want) in cases {
        let Some(got) = mnemonics("yue-Latn-jyutping", text) else {
            eprintln!("[SKIP] no data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
