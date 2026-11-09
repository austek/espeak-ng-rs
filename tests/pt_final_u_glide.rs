// `ph_portugal` turns a word-final `U` after the stress into `w` before a
// vowel (`IF isWordEnd AND isAfterStress AND nextPh(isVowel)`).
//
// Values from a locally built upstream `espeak-ng` (`f0fb11a8`).
// Skips when the pt voice data is absent.

use espeak_ng::Translator;

#[test]
fn final_u_glides_before_a_vowel_word() {
    let Ok(t) = Translator::new("pt", None) else {
        eprintln!("[SKIP] no pt data");
        return;
    };
    for (text, want) in [
        ("O gato está em cima", "ʊ ɡˈatw estˌa ˈeɪŋ sˈimɐ"),
        ("muito obrigado", "mwˈiŋtw ˌɔbɹiɡˈadʊ"),
    ] {
        assert_eq!(&t.text_to_ipa(text).unwrap(), want, "{text:?}");
    }
}
