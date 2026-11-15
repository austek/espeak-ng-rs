// tests/dot_before_hyphen.rs
//
// C `TranslateClause` blanks a dot that follows a word when a space or a hyphen comes next, so
// the dot is not pronounced and the hyphen then joins the two words (translate.c:1513).  Expected
// values are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 4] = [
    ("en", "one.-two", "wˈɒntˈuː", "w'0nt'u:"),
    ("en", "one.- two", "wˈɒn tˈuː", "w'0n t'u:"),
    ("de", "eins.-zwei", "aɪnstsvˈaɪ", "_|aInstsv'aI"),
    ("ru", "раз.-два", "rˈasdvˈɑ", "@-r'asdv'A"),
];

#[test]
fn dot_between_a_word_and_a_hyphen_is_blanked() {
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
