// tests/dotted_abbrev_genitive_handed_over.rs
//
// The apostrophe in `U.S.'s` stays in the word (translate.c:1487), so a voice that hands the
// run to English hands `'s` over as a word with its apostrophe and English reads it `z`, not
// as the letter `s`.  Expected values are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 5] = [
    ("ru", "U.S.'s", "(en)jˌuːˈɛs z(ru)", "(en)j,u:'Es z(ru)"),
    ("ko", "U.S.'s", "(en)jˌuːˈɛs z(ko)", "(en)j,u:'Es z(ko)"),
    ("zh", "U.S.'s", "(en)jˌuː5ˈɛ5s z(cmn)", "(en)j,u:55'E55s_| z(cmn)_|"),
    ("ar", "U.S.'s", "(en)jˌuːˈɛs z(ar)", "(en)j,u:'Es z(ar)"),
    ("hi", "U.S.'s", "(en)jˌuːˈɛs z(hi)", "(en)j,u:'Es z(hi)"),
];

#[test]
fn genitive_after_handed_over_initials_is_z() {
    for (lang, text, ipa, espeak) in CASES {
        assert_eq!(run(lang, "--ipa", text), ipa, "{lang} --ipa {text}");
        assert_eq!(run(lang, "-x", text), espeak, "{lang} -x {text}");
    }
}
