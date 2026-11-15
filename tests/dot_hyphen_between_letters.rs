// tests/dot_hyphen_between_letters.rs
//
// A dot that stays in C's clause text (after a blank or an embedded command) and the hyphen
// behind it make `a . b`: one-letter words that CheckDottedAbbrev joins and spells, and the
// hyphen is a join, never a "minus" (translate.c:1497, translateword.c:1176).  Expected values
// are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 6] = [
    ("en", "<speak>a<mark name=\"q\"/>.-b</speak>", "ˌeɪbˈiː", ",eIb'i:"),
    ("en", "<speak>I<mark name=\"q\"/>.-x</speak>", "ˌaɪˈɛks", ",aI'Eks"),
    ("pl", "<speak>a<mark name=\"q\"/>.-b</speak>", "ˌabˈɛ", ",ab'E"),
    ("en", "x .-y", "ˌɛkswˈaɪ", ",Eksw'aI"),
    ("el", "<speak>9<mark name=\"q\"/>.-b</speak>", "enˈea telˈia(en)bˈiː(el)", "en'ea tel'i;a(en)b'i:(el)"),
    ("en", "a .-b .-c", "ˌeɪbˌiːsˈiː", ",eIb,i:s'i:"),
];

#[test]
fn dot_hyphen_between_letters_is_a_dotted_run() {
    for (lang, text, ipa, espeak) in CASES {
        assert_eq!(run(lang, "--ipa", text), ipa, "{lang} --ipa {text}");
        assert_eq!(run(lang, "-x", text), espeak, "{lang} -x {text}");
    }
}
