// Ligurian names a consonant with a half-long mark when a stressed vowel with
// flag1 precedes it and the next vowel ends the word (`nextVowel(isWordEnd)` in
// ph_ligurian). Expected strings are from C espeak-ng master f0fb11a8.

use espeak_ng::Translator;

#[test]
fn consonant_before_final_vowel_is_half_long() {
    let Some(t) = Translator::new("lij", None).ok() else {
        eprintln!("[SKIP] no lij data");
        return;
    };
    let cases = [
        ("f", "ˈɛfˑfe"),
        ("k", "kˈapˑa"),
        ("z", "zˈitˑa"),
        ("unna", "ˈinˑa"),
        ("disegno", "disˈeɲˑo"),
        ("euro", "ˈejˑru"),
        ("tenea", "tˈenˑja"),
    ];
    for (word, want) in cases {
        let got = t.text_to_ipa(word).unwrap_or_default();
        assert_eq!(got, want, "{word}");
    }
}
