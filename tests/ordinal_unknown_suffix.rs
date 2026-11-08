// tests/ordinal_unknown_suffix.rs
//
// A number followed by a suffix the language has no `_#<suffix>` entry for is
// not an ordinal: C reads the number, then the letters as a word.  The port
// used to parse "1st" as an English ordinal in every language and, finding no
// entry, speak nothing at all ("1st" in es was just "st", "1º" in en lost the
// number).  Expected output is from C espeak-ng master.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let Ok(t) = Translator::new(lang, None) else {
        eprintln!("[SKIP] no {lang} data");
        return None;
    };
    Some(t.text_to_ipa(text).unwrap())
}

#[test]
fn english_suffix_in_a_language_without_it_keeps_the_number() {
    for (lang, text, expected) in [
        ("es", "1st", "ˈuno ˌesetˈe"),
        ("es", "2nd", "dˈos ˌeneðˈe"),
        ("de", "1st", "ˈaɪns ˌɛstˈeː"),
        ("pt", "1st", "ˈum ˌɛstˈe"),
        ("it", "21st", "ventˈuno ˌɛssetˈɪ"),
    ] {
        if let Some(got) = ipa(lang, text) {
            assert_eq!(got, expected, "{lang} {text}");
        }
    }
}

#[test]
fn iberian_indicator_in_english_keeps_the_number() {
    for (text, number) in [("1º", "wˈɒn"), ("21º", "twˈɛnti wˈɒn"), ("2ª", "tˈuː")] {
        if let Some(got) = ipa("en", text) {
            assert!(got.starts_with(number), "en {text}: {got}");
        }
    }
}
