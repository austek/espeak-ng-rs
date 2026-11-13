// West Armenian takes Armenian's final-syllable stress rule.
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn west_armenian_stresses_like_armenian() {
    if ipa("en", "1").is_none() {
        return;
    }
    let cases = [
        ("hyw", "բարեւ", "pʰaɹˈev"),
        ("hyw", "աշխարհ", "aʃχˈarh"),
        ("hyw", "հայերէն", "hˌaɪeɹˈen"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}
