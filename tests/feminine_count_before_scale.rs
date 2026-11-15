// The count before a scale word takes the feminine form when the language's
// `numbers2` has the bit for that scale (C `LookupNum3`, `1 << thousandplex`):
// Macedonian "dve iljadi". Expected strings are espeak-ng master (f0fb11a8)
// output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn macedonian_two_thousand_is_feminine() {
    let Some(_) = ipa("mk", "1") else { return };
    assert_eq!(ipa("mk", "2024").as_deref(), Some("dvˈe ˈiljadi dvˈaɛsˌɛtitʃˈɛtɪɾˌɪ"));
    assert_eq!(ipa("mk", "2 xyz").as_deref().map(|s| s.starts_with("dvˈa")), Some(true));
}
