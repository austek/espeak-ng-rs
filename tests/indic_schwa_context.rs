// Indic inherent-vowel deletion runs through the `V` phoneme program, which
// tests the phoneme two back (`prev2PhW`). Expected values come from C espeak-ng
// f0fb11a8.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    Translator::new_default(lang).ok().map(|t| t.text_to_ipa(text).unwrap())
}

#[test]
fn hindi_medial_schwa_after_vowel_syllable_is_deleted() {
    let Some(i) = ipa("hi", "आपका") else { return };
    assert_eq!(i, "ˌaːpkˈaː");
    assert_eq!(ipa("hi", "उसके").unwrap(), "ˌʊskˈeː");
    assert_eq!(ipa("hi", "सुपरसेट").unwrap(), "sˌʊpəɾsˈeːʈ");
}

#[test]
fn marathi_medial_schwa_is_deleted() {
    let Some(i) = ipa("mr", "कोंबडी") else { return };
    assert_eq!(i, "kˈo\u{303}bɖˌi");
    assert_eq!(ipa("mr", "दीड वाजला").unwrap(), "dˈiːɖ vˈaːzlˌaː");
}
