// Albanian keeps a word-final voiced consonant voiced: C sets no devoicing for sq.
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn word_final_voiced_consonant_stays_voiced() {
    if ipa("en", "1").is_none() {
        return;
    }
    let cases = [
        ("sq", "xyz", "dzˈyz"),
        ("sq", "yz", "ˈyz"),
        ("sq", "az", "ˈaz"),
        ("sq", "mez", "mˈɛz"),
        ("sq", "yzz", "ˈyzz"),
        ("sq", "dyz", "dˈyz"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}
