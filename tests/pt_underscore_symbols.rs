// A symbol that the list only names as `_<char>` (`_/ b'aQ&`, `?1 _# k,ardin'al`)
// is silent in running text; the name is for spelling the character.
//
// Values from a locally built upstream `espeak-ng` (`f0fb11a8`).
// Skips when the pt voice data is absent.

use espeak_ng::Translator;

#[test]
fn underscore_only_symbols_are_silent_in_text() {
    let Ok(t) = Translator::new("pt", None) else {
        eprintln!("[SKIP] no pt data");
        return;
    };
    for (text, want) in [
        ("a/b", "ɐ bˈe"),
        ("1/2", "ˈum dˈoɪʃ"),
        ("15/03", "kˈiŋzɨ zˈɛɾu tɹˈeʃ"),
        ("#tag", "tˈaɡ"),
        ("#5", "sˈiŋku"),
    ] {
        assert_eq!(&t.text_to_ipa(text).unwrap(), want, "{text:?}");
    }
}
