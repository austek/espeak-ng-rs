// A spaced dash pauses before the next word. C puts the pause straight after
// the number's last sound: the number's own end-of-word marker must not print
// a space in front of it. Expected mnemonics are from C espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(lang: &str, text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join(format!("{lang}_dict")).exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', " "))
}

fn check(cases: &[(&str, &str, &str)]) {
    for (lang, text, want) in cases {
        let Some(got) = mnemonics(lang, text) else {
            eprintln!("[SKIP] no {lang} data");
            return;
        };
        assert_eq!(got, *want, "{lang} {text}");
    }
}

#[test]
fn pause_follows_the_number_without_a_space() {
    check(&[
        ("en", "5 - 3", "f'aIv_:_: Tr'i:"),
        ("en", "12 - 30", "tw'Elv_:_: T'3:ti"),
        ("pt", "5 - 3", "s'iNku_:_: tR'es#"),
        ("ku", "5 - 3", "p'endZ_:_: s'e"),
        ("ku", "12 - 30", "danzd'E#_:_: s'i"),
        ("de", "5 - 3", "f'ynf_:_: dr'aI"),
        ("fr", "5 - 3", "s'E~k_:_: trw'a"),
    ]);
}
