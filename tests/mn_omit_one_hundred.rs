// Mongolian omits "one" before "hundred" (NUM_OMIT_1_HUNDRED).
// Expected values come from C espeak-ng f0fb11a8.

use espeak_ng::Translator;

#[test]
fn mongolian_bare_hundred() {
    let Ok(t) = Translator::new_default("mn") else { return };
    assert_eq!(t.text_to_ipa("100").unwrap(), "zˈU:n");
    assert_eq!(t.text_to_ipa("101").unwrap(), "zˈU:n neɡ");
    assert_eq!(t.text_to_ipa("100000").unwrap(), "zˈU:n mjaŋɡa");
}
