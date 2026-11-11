// A dotted abbreviation inside a sentence is matched against the list's
// multi-word `(e . g .)` entry, as C reads the dots as words.  At the end of a
// clause the final dot is gone and the entry no longer matches.  Expected
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
fn a_dot_that_stays_in_the_clause_lets_the_phrase_match() {
    check(&[
        ("en", "x e.g. a", "ˈɛks fˌɔːɹɛɡzˈampəl ˈeɪ"),
        ("en", "x e.g.a", "ˈɛks fˌɔːɹɛɡzˈampəl ˈeɪ"),
        ("en", "x e.g.ab", "ˈɛks fˌɔːɹɛɡzˈampəl ˈab"),
        ("en", "x e.g.A", "ˈɛks fˌɔːɹɛɡzˈampəl ˈeɪ"),
        ("en", "e.g. a", "fˌɔːɹɛɡzˈampəl ˈeɪ"),
    ]);
}

#[test]
fn a_clause_final_dot_leaves_the_letters_spelled() {
    check(&[
        ("en", "e.g.", "ˌiːdʒˈiː"),
        ("en", "x e. g. a", "ˈɛks ˈiː dʒˈiː ˈeɪ"),
        ("en", "e.g. Hammer", "ˌiːdʒˈiː\nhˈamə"),
    ]);
}
