// Vulgar fraction characters are symbols the dictionary names, not "1/2".
// A language without an entry switches to English for the symbol, as in C.
// Expected strings are C espeak-ng master output.
use espeak_ng::Translator;

fn check(cases: &[(&str, &str, &str)]) {
    for (lang, text, expected) in cases {
        let Ok(t) = Translator::new_default(lang) else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        assert_eq!(&t.text_to_ipa(text).unwrap(), expected, "{lang} {text}");
    }
}

#[test]
fn half_is_read_from_the_dictionary() {
    check(&[
        ("en", "½", "ɐ hˈɑːf"),
        ("nl", "½", "ən hˈɑlf"),
        ("de", "½", "ˈaɪn hˈalp"),
        ("fr", "½", "œ̃dəmˈi"),
        ("it", "½", "ʊn mˈɛdzːo"),
        ("pt", "½", "mˈeɪʊ"),
        ("hu", "½", "fˈeːl"),
    ]);
}

#[test]
fn language_without_an_entry_switches_to_english() {
    check(&[("es", "½", "(en)ɐ hˈɑːf(es)"), ("tr", "½", "(en)ɐ hˈɑːf(tr)")]);
}

#[test]
fn fraction_after_a_number_and_quarters() {
    check(&[
        ("en", "1½", "wˈɒn ɐ hˈɑːf"),
        ("en", "2¾", "tˈuː θɹˈiː kwˈɔːtəz"),
        ("de", "2¾", "tsvˈaɪ dɾˈaɪ fˈiːɾtəl"),
    ]);
}
