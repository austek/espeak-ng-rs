// U+2103 and U+2109 name no sound in any language: the number before them is
// spoken and the sign is dropped. `20 °C` reads the letter C.
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn degree_sign_units_are_silent() {
    if ipa("en", "1").is_none() {
        return;
    }
    let cases = [
        ("en", "℃", ""),
        ("de", "20 ℃", "tsvˈantsɪç"),
        ("fr", "20 ℃", "vˈɛ̃"),
        ("ru", "℉", ""),
        ("es", "5 ℉", "θˈinko"),
        ("ku", "20 ℃", "bˈɪst"),
        ("fr", "20 °C", "vˈɛ̃ dəɡʁˈe sˈe"),
        ("fr", "℃", ""),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}
