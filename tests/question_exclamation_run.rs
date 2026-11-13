// A `?!` run after a word is one clause end: the `!` is not spoken by name.
// Expected strings are from C espeak-ng master f0fb11a8.

use espeak_ng::Translator;

#[test]
fn question_exclamation_run_after_a_word_is_silent() {
    let Some(t) = Translator::new("en", None).ok() else {
        eprintln!("[SKIP] no en data");
        return;
    };
    for (text, want) in [("What?!", "wˈɒt"), ("What!?", "wˈɒt"), ("What??", "wˈɒt")] {
        let got = t.text_to_ipa(text).unwrap_or_default();
        assert_eq!(got.trim(), want, "{text}");
    }
}
