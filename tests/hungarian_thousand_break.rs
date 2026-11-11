// When the leading "one" of a thousand is omitted (hu 1000..1999), C's
// LookupNum3 leaves its end-of-word marker in front, so `-x` prints a space
// after the vowel-initial break. Expected mnemonics are from C espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("hu_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", "hu", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', " "))
}

#[test]
fn omitted_one_leaves_the_break() {
    let cases = [
        ("1000", "_! 'EzER2"),
        ("1001", "_! 'EzER2 EJ"),
        ("1234", "_! 'EzER2 ke:tsa:z hAR2mintsne:J"),
        ("2000", "k'e:tEzER2"),
        ("9999", "k'ilEnts EzER2 kilEntssa:z kilEntsvEnkilEnts"),
    ];
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no hu data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
