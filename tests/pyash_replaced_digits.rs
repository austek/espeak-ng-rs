// A digit that the language's `.replace` table maps to a letter (Pyash 2, 6, 7)
// is a letter before any number check, so it never takes part in a number.
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(text: &str) -> Option<String> {
    let t = Translator::new_default("py").ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn replaced_digits_are_letters() {
    let Some(_) = ipa("1") else { return };
    let cases = [
        ("2", "ˈu"),
        ("26", "ˈu ə"),
        ("ba2", "bˈau"),
        ("a7", "ˈai"),
        ("12 abc", "hjˈik ˈu ˈabʃ"),
    ];
    for (text, want) in cases {
        assert_eq!(ipa(text).as_deref(), Some(want), "py {text}");
    }
}
