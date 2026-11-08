// A spelled-out word gets one break at its start, not one before every
// vowel-initial letter name.  Expected mnemonics are from C espeak-ng master.

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
fn spelled_letters_get_one_break_at_the_start_of_the_word_only() {
    let cases = [
        ("kmo", "k,a:,Em'o:"),
        ("KMI", "k,a:,Em'i:"),
        ("VMMa", "f,e:,Em,Em'a:"),
        ("brv", "b,e:,Erf'e:"),
        ("rng", "_!,Er,EnQ'e:"),
        ("nvda", "_!,Enf,e:d,e:'a:"),
        ("mnr", "_!,Em,En'Er"),
        ("mcn", "_!,Ems,e:'En"),
        ("lcd", "_!,Els,e:d'e:"),
    ];
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no nl data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
