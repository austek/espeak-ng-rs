// Hungarian reads `3.` before a lower-case word as an ordinal, and a number
// followed by a hyphenated `_#-` suffix as an ordinal with that suffix.
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(text: &str) -> Option<String> {
    Translator::new_default("hu").ok()?.text_to_ipa(text).ok()
}

#[test]
fn dot_and_hyphen_ordinals() {
    if ipa("1").is_none() {
        return;
    }
    let cases = [
        ("3. haus", "hˈɑrmɑdik hˈɑuʃ"),
        ("21.-én", "hˈusonɛɟːɛdikeːn"),
        ("3-án", "hˈɑrmɑdikaːn"),
    ];
    for (text, want) in cases {
        let got = ipa(text);
        assert_eq!(got.as_deref().map(str::trim), Some(want), "hu {text}");
    }
}
