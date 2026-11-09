// The word-final rule `@) o (_ =U` puts primary stress on the preceding
// vowel and demotes an earlier primary mark of the rules (`?1 C) e ['i`) to
// secondary: `leopoldo` is `lˌiupˈoldʊ`, not two primaries.  `$alt2` then
// closes the `ɔ` of that last primary.
//
// Values from a locally built upstream `espeak-ng` (`f0fb11a8`).
// Skips when the pt voice data is absent.

use espeak_ng::Translator;

fn check(lang: &str, cases: &[(&str, &str)]) {
    let Ok(t) = Translator::new(lang, None) else {
        eprintln!("[SKIP] no {lang} data");
        return;
    };
    for (text, want) in cases {
        assert_eq!(&t.text_to_ipa(text).unwrap(), want, "{lang} {text:?}");
    }
}

#[test]
fn final_prev_stress_demotes_the_earlier_primary() {
    check(
        "pt",
        &[
            ("leopoldo", "lˌiupˈoldʊ"),
            ("Leopoldo", "lˌiupˈoldʊ"),
            ("leopoldos", "lˌiupˈoldʊʃ"),
            ("leopardo", "lˌiupˈaɾədʊ"),
            ("teopoldo", "tˌiupˈɔldʊ"),
            ("leopolda", "lˌiupˈɔldɐ"),
            ("teodoro", "tˌiudˈɔɾʊ"),
            ("geometria", "ʒˌiumɨtɹˈiɐ"),
        ],
    );
}
