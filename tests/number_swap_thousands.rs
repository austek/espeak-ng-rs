// Bishnupriya and Oromo say the scale word before its count (NUM2_SWAP_THOUSANDS).
// Expected values come from C espeak-ng f0fb11a8.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    Translator::new_default(lang).ok().map(|t| t.text_to_ipa(text).unwrap())
}

#[test]
fn bishnupriya_scale_word_first() {
    let Some(i) = ipa("bpy", "1000") else { return };
    assert_eq!(i, "lˈisiŋ\u{303} cˌamma");
    assert_eq!(ipa("bpy", "2024").unwrap(), "lˌisiŋ\u{303} tʃˌaːni kuːnmˈaːɾi");
    assert_eq!(ipa("bpy", "1000000").unwrap(), "lˈakʰ ʈˈaːɾa");
}

#[test]
fn oromo_scale_word_first() {
    let Some(i) = ipa("om", "1000") else { return };
    assert_eq!(i, "kˌuma tˈokːo");
    assert_eq!(ipa("om", "2024").unwrap(), "kˌuma lˌama fˌi diɡdˈamiːˌafur");
    assert_eq!(ipa("om", "1000000").unwrap(), "mˌilijˌoniː tˈokːo");
}
