// tests/dot_after_command_before_hyphen.rs
//
// C blanks a dot before a hyphen only when the previous source character is a letter
// (translate.c:1513); after an embedded command (SSML mark, break) it stays, is read, and the
// hyphen joins it to the next word.  Expected values are what C espeak-ng master (f0fb11a8)
// prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).replace('\n', " ").trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 5] = [
    ("en", "<speak>word<mark name=\"m\"/>.-next</speak>", "wˈɜːd dˈɒtnˈɛkst", "w'3:d d'0tn'Ekst"),
    ("it", "<speak>word<mark name=\"m\"/>.-next</speak>", "wˈɔrd nˈekst", "w2'O@-*d _|n'ekst"),
    ("en", "<speak>a <mark name=\"m\"/>.-next</speak>", "ˈeɪ dˈɒtnˈɛkst", "'eI d'0tn'Ekst"),
    ("en", "<speak>word<break time=\"1ms\"/>.-next</speak>", "wˈɜːd dˈɒtnˈɛkst", "w'3:d d'0tn'Ekst"),
    ("en", "<speak>word<mark name=\"m\"/> .-next</speak>", "wˈɜːd dˈɒtnˈɛkst", "w'3:d d'0tn'Ekst"),
];

#[test]
fn dot_after_a_command_before_a_hyphen_is_read() {
    for (lang, text, ipa, espeak) in CASES {
        assert_eq!(run(lang, "--ipa", text), ipa, "{lang} --ipa {text}");
        assert_eq!(run(lang, "-x", text), espeak, "{lang} -x {text}");
    }
}
