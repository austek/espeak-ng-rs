// Bengali keeps the word-final inherent vowel; only the V phoneme program
// deletes it. Expected values come from C espeak-ng f0fb11a8.

use espeak_ng::Translator;

#[test]
fn bengali_keeps_final_vowel() {
    let Ok(t) = Translator::new_default("bn") else { return };
    assert_eq!(t.text_to_ipa("হার্ট সহ দম্পতি").unwrap(), "hˈaɾʈɔ ʃˈɔho dˈɔmpɔtˌi");
    assert_eq!(t.text_to_ipa("সবুজ বর্গক্ষেত্র").unwrap(), "ʃˈobuɟ bˈɔɾɡɔkʰkʰˌetɾɔ");
}
