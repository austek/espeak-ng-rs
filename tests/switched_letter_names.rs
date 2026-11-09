// tests/switched_letter_names.rs
//
// A lone Latin letter in a Cyrillic voice is handed to English, where it is
// spelled by its letter name ("ligature a e"), not read as a word, and
// without English's own `.replace` table (`æ` -> `ae`).  Expected output is
// from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn a_switched_single_letter_is_spelled() {
    let cases = [
        ("ru", "æ", "(en)lˌɪɡɐtʃəeɪˈiː(ru)"),
        ("ru", "Æ", "(en)lˌɪɡɐtʃəeɪˈiː(ru)"),
        ("ru", "œ", "(en)lˌɪɡɐtʃəəʊˈiː(ru)"),
        ("ru", "A", "(en)ˈeɪ(ru)"),
        ("ru", "I", "(en)ˈaɪ(ru)"),
        ("ru", "Ł", "(en)ˌɛlstɹˈəʊk(ru)"),
        ("ru", "я æ я", "ˈja (en)lˌɪɡɐtʃəeɪˈiː(ru) ˈja"),
        ("uk", "æ", "(en)lˌɪɡɐtʃəeɪˈiː(uk)"),
        ("uk", "A", "(en)ˈeɪ(uk)"),
        ("bg", "œ", "(en)lˌɪɡɐtʃəəʊˈiː(bg)"),
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
