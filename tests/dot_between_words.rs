// A '.' between two words is a word of its own that the language names (`_.`,
// `.` or a `.` rule): es "punto", sv "punkt", en "dot".  A word whose list entry
// carries `$dot` swallows the dot after it, and a lone letter before the dot is
// spelled.  Expected strings are C espeak-ng master output.
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
fn a_dot_between_words_is_named_when_the_language_names_it() {
    check(&[
        ("es", "cat.dog", "kˈat pˈunto ðˈoɡ"),
        ("sv", "t.ex.", "tˈeː pˈɵŋt ˈɛks"),
        ("sv", "t.ex. äpplen", "tˈeː pˈɵŋt ˈɛks pˈɵŋt ˈɛplən"),
        ("es", "ab.cd. ef", "ˈab pˈunto θˌeðˈe pˈunto ˈef"),
    ]);
}

#[test]
fn a_word_that_allows_a_dot_drops_it() {
    check(&[
        ("es", "p.ej. manzanas", "pˈe pˈunto exˈemplo manθˈanas"),
        ("pt", "Sr.ª Silva", "ˌɛsʲˈɛrɹ ˌɔɾədinˈalfeminˈi\u{303}nʊ sˈilvɐ"),
    ]);
}

#[test]
fn a_lone_letter_before_the_dot_is_spelled() {
    check(&[
        ("en", "a.cat", "ˈeɪ dˈɒt kˈat"),
        ("fr", "a.bc", "ˈa bˌesˈe"),
        ("it", "a.casa", "ˈaː kˈaza"),
    ]);
}
