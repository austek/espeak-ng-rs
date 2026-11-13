// The neutral tone takes its level from the tone before it: 22 after 55, 33
// after 35, 44 after 214. Expected mnemonics are from C espeak-ng master.

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
fn neutral_tone_level_follows_the_previous_tone() {
    let cases = [
        ("华夫饼", "Xw'A35_| fu33_| p'i214N_|"),
        (
            "做成心形的双手",
            "ts'uo51_| ts.h'@35N_| S;'i55n_| S;'i35N_| t@33_| s.wA55Ns.'ou214_|",
        ),
        ("棺材", "kwa55ntshai22_|"),
    ];
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
