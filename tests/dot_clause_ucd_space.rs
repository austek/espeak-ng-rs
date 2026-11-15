// tests/dot_clause_ucd_space.rs
//
// C `ReadClause` tests blanks with `iswspace`, which espeak-ng maps to `ucd_isspace`.  That
// class leaves out U+00A0, U+2007 and U+202F, so a dot followed by one of them is not a clause
// end and is read as the word "dot" (readclause.c:817, 854).  Expected values are what C
// espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 10] = [
    ("en", "Hi.\u{a0}there", "hˈaɪ dˈɒt ðˈeə", "h'aI d'0t D'e@"),
    ("en", "one.\u{2007}two", "wˈɒn dˈɒt tˈuː", "w'0n d'0t t'u:"),
    ("en", "one.\u{202f}two", "wˈɒn dˈɒt tˈuː", "w'0n d'0t t'u:"),
    ("en", "one.\u{a0}\u{a0}two", "wˈɒn dˈɒt tˈuː", "w'0n d'0t t'u:"),
    ("en", "one. \u{a0}two", "wˈɒn\ntˈuː", "w'0n\nt'u:"),
    ("en", "Hi.\u{a0}There", "hˈaɪ dˈɒt ðˈeə", "h'aI d'0t D'e@"),
    ("nl", "Hi.\u{a0}there", "hˈi pˈɵn tˈɪːrə", "h'i p'8n! t'I:r@"),
    ("sv", "Hi.\u{a0}there", "hˈiː pˈɵŋt thˈeːrə", "h'i: p'8Nt th'e:RE-"),
    ("hu", "Hi.\u{a0}there", "hˈi pont thˈɛrɛ", "h'i pont th'ERE"),
    ("mto", "one .\u{a0}two", "oˈn̩e puˈn̩to tʰ ˌuβe dˈoβle ˈo", "o'ne pu'nto t# ,uBe d'oBle 'o"),
];

#[test]
fn dot_before_a_no_break_space_stays_in_the_clause() {
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
