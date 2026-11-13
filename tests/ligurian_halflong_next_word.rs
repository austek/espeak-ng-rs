// A phoneme program's `nextPhW(isWordEnd)` must see a word boundary two
// phonemes ahead, also when the next word follows. Expected strings are from C
// espeak-ng master f0fb11a8.

use espeak_ng::Translator;

#[test]
fn half_long_nasal_before_a_final_vowel_with_a_following_word() {
    let Some(t) = Translator::new("lij", None).ok() else {
        eprintln!("[SKIP] no lij data");
        return;
    };
    let cases = [
        ("stelliñe schillente", "stelˈiŋˑe skilˈeŋte"),
        (
            "persoña inte unna stansia piña de vapô",
            "persˈuŋˑa ˈiŋte na stˈaŋsja pˈiŋˑa de vapˈuː",
        ),
    ];
    for (text, want) in cases {
        let got = t.text_to_ipa(text).unwrap_or_default();
        assert_eq!(got, want, "{text}");
    }
}
