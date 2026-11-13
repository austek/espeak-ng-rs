// A Cyrillic word containing a letter outside the language's letter table is
// still spoken (spelled letter by letter), never dropped. Expected strings are
// espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn unknown_letter_words_are_not_silent() {
    if ipa("en", "1").is_none() {
        return;
    }
    let cases = [
        ("ru", "дѣ", "dˌɛɭˌitətʃʲɪtˈyrʲɪʃˌɛsʲtʲtrʲˈi"),
        ("ru", "бѣ", "bˌɛɭˌitətʃʲɪtˈyrʲɪʃˌɛsʲtʲtrʲˈi"),
        ("ru", "дꙋм", "dˌɛɭˌitəˈeːjʃˌɛsʲtʲtʃʲɪtˌyrʲɪbˌiːˈɛm"),
        ("ru", "дᲀм", "dˌɛɭˌitəʌdʲˈinsˌiːvˌʌsʲimʲnˌʌɭˈɛm"),
        ("ru", "хлѣбъ", "xˌɑˌɛɭɭˈetətʃʲɪtˌyrʲɪʃˌɛsʲtʲtrʲˈibˌɛtvʲˌʌrdyjznˈɑk"),
        ("bg", "ꙋм", "bˌukvɐˌeːjʃˈesttʃˌetirˌibˌiːmˈə"),
        ("uk", "дѣ", "dˌɛlˌetətʃotˈereʃˌiːsttrˈɪ"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}
