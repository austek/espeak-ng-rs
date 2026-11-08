// Letters spelled out inside one word run together: only the first letter name
// gets the vowel-initial pause, so `r` before `n` stays `r` instead of `ɾ`.
// Expected strings come from C espeak-ng master (f0fb11a).

use std::path::Path;
use std::process::Command;

fn mnemonics(text: &str) -> String {
    let bin = env!("CARGO_BIN_EXE_espeak_cli");
    let out = Command::new(bin)
        .args(["-q", "-x", "-v", "de", "--", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn check(cases: &[(&str, &str)]) {
    if !Path::new("espeak-ng-data/de_dict").exists() {
        eprintln!("[SKIP] no de data");
        return;
    }
    for (text, want) in cases {
        assert_eq!(&mnemonics(text), want, "de {text:?}");
    }
}

#[test]
fn spelled_abbreviation_has_one_leading_pause() {
    check(&[
        ("rn", "_!,ER'En"),
        ("rng", "_!,ER,Eng'e:"),
        ("mcn", "_!,Emts,e:'En"),
        ("ndr", "_!,End,e:'Er"),
        ("ich rn", "_|IC _!,ER'En"),
        ("tld", "t,e:,Eld'e:"),
        ("grv", "g,e:,Erf'aU"),
        ("bmw", "b,e:,Emv'e:"),
    ]);
}

#[test]
fn single_letters_and_words_keep_their_pause() {
    check(&[
        ("r", "_!'Er"),
        ("n", "_!'En"),
        ("r n", "_!'Er _!'En"),
        ("ach", "_!'ax"),
        ("ard", "_!'art"),
    ]);
}
