// tests/dotted_ordinals.rs
//
// A dot between a number and the º/ª indicator ("1.º", "2.ª") is not an
// ordinal marker in C: the dot is the language's `_.` entry appended to the
// cardinal, and the indicator is read as the letter.  Expected strings are
// C espeak-ng master output.

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
fn dot_before_the_indicator_reads_cardinal_point_letter() {
    check(&[
        ("es", "3.º", "tɾˈes punto ˈo"),
        ("it", "3.º", "trˈe pʊnto ˈɔː"),
        ("de", "3.º", "dɾˈaɪ pʊŋkt ˈoː"),
        ("de", "3.ª", "dɾˈaɪ pʊŋkt ˈɑː"),
        ("en", "3.º", "θɹˈiː dɒt ˈəʊ"),
    ]);
}

#[test]
fn bare_dot_stays_a_cardinal() {
    check(&[
        ("es", "3.", "tɾˈes"),
        ("de", "3.", "dɾˈaɪ"),
        ("nl", "3.", "drˈi"),
        ("nl", "Hij is 3.", "hɛɪ ɪs drˈi"),
        ("nl", "Kerkstraat 12.", "kˈɛrkstraː tʋˈaːlf"),
    ]);
}
