// A word of a non-Latin alphabet with no vowel within the language's initial
// consonant limit is spelled letter by letter, using the vowels of that
// alphabet's letter groups. Expected strings are espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn vowelless_non_latin_words_are_spelled() {
    if ipa("en", "1").is_none() {
        return;
    }
    let cases = [
        ("ru", "ФСБ", "ˌɛfˌɛsbˈɛ"),
        ("ru", "ФБР", "ˌɛvbˌɛˈɛr"),
        ("ru", "СССР", "ˌɛsˌɛsˌɛsˈɛr"),
        ("ru", "кг", "kˌɑɡˈɛ"),
        ("el", "ΒΓΔ", "vˌitaɣˌamaðˈelta"),
        ("bg", "ΒΓΔ", "(el)vˌitaɣˌamaðˈelta(bg)"),
        ("mk", "БВГД", "bˌəvˌəɡˌədˈə"),
        ("ka", "ააა", "ˈaaa"),
        ("ka", "სსს", "sˌəsˌəsˈə"),
        ("ka", "გგგ", "ɡˌəɡˌəɡˈə"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}
