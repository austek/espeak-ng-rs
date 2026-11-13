// A Mandarin syllable with the neutral tone (`11`) carries no stress mark, even
// when it is the only or the last syllable of its word. Expected mnemonics are
// from C espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("cmn_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", "cmn", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', "/"))
}

#[test]
fn neutral_tone_syllables_are_unstressed() {
    let cases = [
        ("坊", "fA11N_|"),
        ("棺材", "kwa55ntshai22_|"),
        ("地球上的美洲", "ti51tS;h'iou35_| s.'A51N_| t@11_| m'ei21_| ts.'ou55_|"),
        ("钥匙和锁", "j'Au51_| s.i.11_| X'o-35_| s'uo214_|"),
    ];
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
