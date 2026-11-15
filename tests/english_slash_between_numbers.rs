// English reads a slash between numbers as "slash"; C has no fraction
// rewriting for the ASCII form (`1/2` is "one slash two"), while the vulgar
// fraction characters come from the dictionary. Expected strings are
// espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(text: &str) -> Option<String> {
    let t = Translator::new_default("en").ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn slash_between_numbers_is_spoken() {
    let Some(_) = ipa("1") else { return };
    let cases = [
        ("24/7", "twˈɛnti fˈɔː slˈaʃ sˈɛvən"),
        ("1/2", "wˈɒn slˈaʃ tˈuː"),
        ("3/4", "θɹˈiː slˈaʃ fˈɔː"),
        ("5/8", "fˈaɪv slˈaʃ ˈeɪt"),
    ];
    for (text, want) in cases {
        assert_eq!(ipa(text).as_deref(), Some(want), "en {text}");
    }
}

#[test]
fn vulgar_fractions_still_read_from_the_dictionary() {
    let Some(_) = ipa("1") else { return };
    assert_eq!(ipa("½").as_deref(), Some("ɐ hˈɑːf"));
    assert_eq!(ipa("¾").as_deref(), Some("θɹˈiː kwˈɔːtəz"));
    assert_eq!(ipa("1½").as_deref(), Some("wˈɒn ɐ hˈɑːf"));
}
