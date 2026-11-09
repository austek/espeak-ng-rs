// Single letters joined by dots (`e.g.`, `U.S.A.`, `a.u.b.`) are one word in C,
// spelled with a secondary stress on every letter but the last.  Expected
// strings are C espeak-ng master output.
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
fn dotted_letters_are_spelled_as_one_word() {
    check(&[
        ("en", "e.g.", "ˌiːdʒˈiː"),
        ("en", "U.S.A.", "jˌuːˌɛsˈeɪ"),
        ("en", "the U.S.A. is big", "ðə jˌuːˌɛsˈeɪ ɪz bˈɪɡ"),
        ("nl", "a.u.b.", "ˌaːybˈeː"),
        ("nl", "d.w.z.", "dˌeːʋˌeːzˈɛt"),
        ("nl", "U.S.A.", "yˌɛsˈaː"),
        ("de", "z.B.", "tsˌɛtbˈeː"),
        ("de", "d.h.", "dˌeːhˈɑː"),
    ]);
}

#[test]
fn dictionary_entry_for_the_dotted_word_wins() {
    check(&[("en", "i.e.", "ˈaɪiː")]);
}
