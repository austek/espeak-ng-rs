// Macedonian replaces the vowel letter groups with a list that includes р, so
// a word whose only "vowel" is р is a word, not a spelled acronym. Expected
// strings are espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn mk_r_counts_as_a_vowel() {
    if ipa("en", "1").is_none() {
        return;
    }
    let cases = [
        ("mk", "крв", "kˈr̩f"),
        ("mk", "прст", "pˈr̩st"),
        ("mk", "врв", "vˈr̩f"),
        ("mk", "дрво", "dˈr̩vo"),
        ("mk", "српски", "sˈr̩pskɪ"),
        ("mk", "вртлог", "vˈr̩tloɡ"),
        ("mk", "црн", "tsˈr̩n"),
        ("mk", "брзо", "bˈr̩zo"),
        ("mk", "страна", "strˈanæ"),
        ("mk", "Македонија", "mˌækedˈonɪjˌæ"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}
