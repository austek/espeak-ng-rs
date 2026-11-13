// A Cantonese list entry written with its tone after the final consonant
// (`baat3`) puts the tone on the syllable's vowel, with no second default tone.
// Expected mnemonics are from C espeak-ng master.

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
fn list_entry_tone_follows_the_vowel() {
    let cases = [
        ("金", "g'a1m_|"),
        ("晚", "m'aa5n_|"),
        ("再见", "z'oi3_| g'i3n_|"),
        ("对不起", "d'eoi3_| b'a1t_| h'ei2_|"),
        ("没关系", "m'u6t_| gw'aa1n_| h'ai6_|"),
        ("今天天气很好", "g'a1m_| t'i1n_| t'i1n_| h'ei3_| h'a2n_| h'ou2_|"),
    ];
    for (text, want) in cases {
        for lang in ["yue", "yue-Latn-jyutping"] {
            let Some(got) = mnemonics(lang, text) else {
                eprintln!("[SKIP] no data");
                return;
            };
            assert_eq!(got, want, "{lang} {text}");
        }
    }
}
