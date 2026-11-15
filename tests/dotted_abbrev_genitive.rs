// tests/dotted_abbrev_genitive.rs
//
// C `CheckDottedAbbrev` deletes the final dot of `u.s.a.'s` and ReadClause keeps that dot in the
// clause (readclause.c:872), so the initials join and the `'s` is a word of its own.  Expected
// values are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 10] = [
    ("en", "u.s.a.'s plan", "jˌuːˌɛsˈeɪ z plˈan", "j,u:,Es'eI z pl'an"),
    ("en", "u . s . a .'s plan", "jˌuːˌɛsˈeɪ z plˈan", "j,u:,Es'eI z pl'an"),
    ("en", "u.s.a.'s", "jˌuːˌɛsˈeɪ z", "j,u:,Es'eI z"),
    ("en", "x . y .'s", "ˌɛkswˈaɪ z", ",Eksw'aI z"),
    ("en", "a . b . f .'s plan", "ˌeɪbˌiːˈɛf z plˈan", ",eIb,i:;'Ef z pl'an"),
    ("en", "a . b .'s dog", "ˌeɪbˈiː z dˈɒɡ", ",eIb'i: z d'0g"),
    ("de", "a . b .'s plan", "ˌɑːbˈeː s plˈɑːn", "_!,A:b'e: s_:_: pl'A:n"),
    ("es", "a . b .'s plan", "ˌaβˈe s plˈan", ",aB'e s_:_: pl'an"),
    ("it", "a . b .'s plan", "ˌaːbˈi s plˈan", ",a:b'i s pl'an"),
    ("pl", "u.s.a.'s plan", "ˌuˌɛsˈa s plˈan", ",u,Es'a s_:_: pl'an_"),
];

#[test]
fn genitive_after_dotted_initials() {
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
