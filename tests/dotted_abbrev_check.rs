// tests/dotted_abbrev_check.rs
//
// C `CheckDottedAbbrev` runs after a multi-word list entry has had its chance: a one-letter
// word followed by `. ` and more one-letter words is copied without the dots and spelled
// (`a . b` is `ab`).  When the language hands the letter to another voice, that voice reads
// `ab` as a word.  Expected values are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 26] = [
    ("en", "a . b", "ˌeɪbˈiː", ",eIb'i:"),
    ("en", "x . y z", "ˌɛkswˈaɪ zˈɛd", ",Eksw'aI z'Ed"),
    ("en", "a . b . c", "ˌeɪbˌiːsˈiː", ",eIb,i:s'i:"),
    ("en", "<speak>a <emphasis>.</emphasis> b</speak>", "ˌeɪbˈiː", ",eIb'i:"),
    ("en", "<speak>x<mark name=\"m\"/>. y</speak>", "ˌɛkswˈaɪ", ",Eksw'aI"),
    ("de", "a . b", "ˌɑːbˈeː", "_!,A:b'e:"),
    ("es", "a . b", "ˌaβˈe", ",aB'e"),
    ("ru", "a . b", "(en)ˈab(ru)", "(en)'ab(ru)"),
    ("zh", "a . b", "(en)ˈa5b(cmn)", "(en)'a55b(cmn)_|"),
    ("pt", "x . y . z", "ʃˌiʒˌipsˌilɔŋzˈe", "S,iZ,ips,ilONz'e"),
    ("en", "e . g . it", "fˌɔːɹɛɡzˈampəl ɪt", "f,O@rEgz'aamp@L It"),
    ("en", "a . b . x y", "ˌeɪbˌiːˈɛks wˈaɪ", ",eIb,i:;'Eks w'aI"),
    ("en", "a.b . c", "ˌeɪbˌiːsˈiː", ",eIb,i:s'i:"),
    ("en", "ab . c . d", "ˈab dˈɒt sˌiːdˈiː", "'ab d'0t s,i:d'i:"),
    ("en", "a . dog", "ˈeɪ dˈɒt dˈɒɡ", "'eI d'0t d'0g"),
    ("en", "a . bc", "ˈeɪ dˈɒt bˌiːsˈiː", "'eI d'0t b,i:s'i:"),
    ("en", "the a . b cat", "ðɪ ˌeɪbˈiː kˈat", "DI2; ,eIb'i: k'at"),
    ("en", "a . b . c . d", "ˌeɪbˌiːsˌiːdˈiː", ",eIb,i:s,i:d'i:"),
    ("fr", "a . b", "ˌabˈe", ",ab'e"),
    ("it", "a . b . c", "ˌaːbˌitʃˈi", ",a:b,itS'i"),
    ("nl", "a . b", "ˌaːbˈeː", "_!,a:b'e:"),
    ("ar", "a . b", "(en)ˈab(ar)", "(en)'ab(ar)"),
    ("ku", "a . b", "ˌabˈe", ",ab'e"),
    ("hi", "a . b", "(en)ˈab(hi)", "(en)'ab(hi)"),
    ("ru", "а . б", "ˌabˈɛ", ",ab'E"),
    ("pt", "x . b", "ʃˌiʒbˈe", "S,iZb'e"),
];

#[test]
fn dotted_initials_join_as_c_checks_them() {
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
