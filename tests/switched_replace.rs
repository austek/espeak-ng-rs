// tests/switched_replace.rs
//
// The `.replace` table belongs to the language that reads the clause.  A word
// handed to English is not substituted again (`æ` -> `ae`): English reads the
// original letters, where an unknown accented letter loses its accent.
// Expected output is from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn a_switched_word_keeps_its_original_letters() {
    let cases = [
        ("ru", "aæb", "(en)ˈɑːb(ru)"),
        ("ru", "ææ", "(en)ˈɑː(ru)"),
        ("ru", "xœy", "(en)zˈɔɪ(ru)"),
        ("uk", "aæb", "(en)ˈɑːb(uk)"),
        ("uk", "xœy", "(en)zˈɔɪ(uk)"),
        ("bg", "ææ", "(en)ˈɑː(bg)"),
        ("bg", "xœy", "(en)zˈɔɪ(bg)"),
    ];
    let mut failures = Vec::new();
    for (lang, text, expected) in cases {
        let Ok(t) = Translator::new(lang, None) else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let got = t.text_to_ipa(text).unwrap();
        if got != expected {
            failures.push(format!("{lang} {text:?}: got {got:?}, C says {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
