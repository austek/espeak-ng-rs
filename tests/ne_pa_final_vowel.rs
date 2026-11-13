// Nepali and Punjabi keep the word-final inherent vowel (C espeak-ng f0fb11a8).

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    Translator::new_default(lang).ok().map(|t| t.text_to_ipa(text).unwrap())
}

#[test]
fn nepali_keeps_final_schwa() {
    let Some(i) = ipa("ne", "अधिकार") else { return };
    assert_eq!(i, "ˌədʰɪkˈaːɾə");
    assert_eq!(ipa("ne", "तीर").unwrap(), "tˈɪːɾə");
}

#[test]
fn punjabi_keeps_final_vowel() {
    let Some(i) = ipa("pa", "ਅ") else { return };
    assert_eq!(i, "ˈɛrʌ");
    assert_eq!(ipa("pa", "ੳ").unwrap(), "ˈʊrʌ");
}
