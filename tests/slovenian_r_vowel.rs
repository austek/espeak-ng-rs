// For the "cannot be pronounced" test C counts `r` as a vowel in Slovenian,
// so a word of consonants and `r` is read, not spelled.
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn slovenian_word_with_only_r_as_vowel_is_read() {
    if ipa("en", "1").is_none() {
        return;
    }
    assert_eq!(ipa("sl", "pfrt").as_deref(), Some("pfˈərt"));
}
