// tests/embedded_command_empty_word_pause.rs
//
// C `TranslateWord2` gives any word that has no phonemes a very short pause when an embedded
// command is attached to it, so the command has something to land on (translate.c:649).  The
// rule covers every phoneme-less word, not only a dot.  Expected values are what C espeak-ng
// master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 9] = [
    ("en", "<speak>cat <mark name=\"m\"/>_ dog</speak>", "kˈat dˈɒɡ", "k'at d'0g"),
    ("de", "<speak>Hund <mark name=\"m\"/>_ Katze</speak>", "hˈʊnt kˈatsə", "h'Unt k'ats@"),
    ("fr", "<speak>chat <mark name=\"m\"/>_ chien</speak>", "ʃˈa ʃjˈɛ̃", "S'a Sj'E~"),
    ("en", "<speak>cat <mark name=\"m\"/>__ dog</speak>", "kˈat dˈɒɡ", "k'at d'0g"),
    ("en", "<speak>cat <mark name=\"m\"/>( dog</speak>", "kˈat  dˈɒɡ", "k'at _|_:_: d'0g"),
    ("en", "<speak>cat <mark name=\"m\"/>\" dog</speak>", "kˈat  dˈɒɡ", "k'at _|_:_: d'0g"),
    ("en", "<speak>cat <mark name=\"m\"/>' dog</speak>", "kˈat dˈɒɡ", "k'at_:_: d'0g"),
    ("de", "<speak>Hund <mark name=\"m\"/>( Katze</speak>", "hˈʊnt  kˈatsə", "h'Unt _|_:_: k'ats@"),
    ("es", "<speak>perro <mark name=\"m\"/>\" gato</speak>", "pˈero  ɡˈato", "p'eR2o _|_:_: g'ato"),
];

#[test]
fn phonemeless_word_with_embedded_command_gets_a_pause() {
    let mut failures = Vec::new();
    for (lang, text, ipa, x) in CASES {
        for (flag, want) in [("--ipa", ipa), ("-x", x)] {
            let got = run(lang, flag, text);
            if got != want {
                failures.push(format!("{lang} {flag} {text:?}\n  want {want:?}\n  got  {got:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
