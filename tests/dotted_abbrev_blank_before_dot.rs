// tests/dotted_abbrev_blank_before_dot.rs
//
// C reads `a .b` (a blank before the dot, none after) like `a . b`: the dot is punctuation and
// ReadClause puts a blank behind it, so CheckDottedAbbrev joins the initials.  Expected values
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

const CASES: [Case; 19] = [
    ("en", "a .b", "ˌeɪbˈiː", ",eIb'i:"),
    ("en", "A .B .C", "ˌeɪbˌiːsˈiː", ",eIb,i:s'i:"),
    ("en", "x .y z", "ˌɛkswˈaɪ zˈɛd", ",Eksw'aI z'Ed"),
    ("en", "e .g it", "ˌiːdʒˈiː ɪt", ",i:dZ'i:; It"),
    ("en", "I .e", "ˈaɪiː", "'aIi:_!"),
    ("en", "a  .b", "ˌeɪbˈiː", ",eIb'i:"),
    ("en", "the a .b cat", "ðɪ ˌeɪbˈiː kˈat", "DI2; ,eIb'i: k'at"),
    ("en", "a .b .c", "ˌeɪbˌiːsˈiː", ",eIb,i:s'i:"),
    ("de", "a .b", "ˌɑːbˈeː", "_!,A:b'e:"),
    ("es", "a .b", "ˌaβˈe", ",aB'e"),
    ("fr", "a .b", "ˌabˈe", ",ab'e"),
    ("it", "a .b", "ˌaːbˈi", ",a:b'i"),
    ("nl", "a .b", "ˌaːbˈeː", "_!,a:b'e:"),
    ("pt", "a .b", "ˌɐbˈe", ",&b'e"),
    ("pl", "a .b", "ˌabˈɛ", ",ab'E"),
    ("sv", "a .b", "ˌɑːbˈeː", ",A:b'e:"),
    ("hu", "a .b", "ˈɑːbˌeː", "_!'A:b,e:"),
    ("ka", "a .b", "(en)ˈab(ka)", "(en)'ab(ka)"),
    ("ru", "a .b", "(en)ˈab(ru)", "(en)'ab(ru)"),
];

#[test]
fn blank_before_dot_joins_initials() {
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
