// tests/handed_over_word_case.rs
//
// C hands a word to another language's translator with its case intact, so an all-capitals
// word reaches that language's `$allcaps` entries (`US` is spelled, `us` is a pronoun).
// Expected values are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 7] = [
    ("ru", "US", "(en)jˌuːˈɛs(ru)", "(en)j,u:'Es(ru)"),
    ("uk", "US", "(en)jˌuːˈɛs(uk)", "(en)j,u:'Es(uk)"),
    ("ar", "US", "(en)jˌuːˈɛs(ar)", "(en)j,u:'Es(ar)"),
    ("ru", "US x", "(en)jˌuːˈɛs ˈɛks(ru)", "(en)j,u:'Es 'Eks(ru)"),
    ("ru", "OK now", "(en)ˌəʊkˈeɪ nˈaʊ(ru)", "(en),oUk'eI n'aU(ru)"),
    ("el", "US", "(en)jˌuːˈɛs(el)", "(en)j,u:'Es(el)"),
    ("ko", "US", "(en)jˌuːˈɛs(ko)", "(en)j,u:'Es(ko)"),
];

#[test]
fn handed_over_word_keeps_its_case() {
    for (lang, text, ipa, espeak) in CASES {
        assert_eq!(run(lang, "--ipa", text), ipa, "{lang} --ipa {text}");
        assert_eq!(run(lang, "-x", text), espeak, "{lang} -x {text}");
    }
}
