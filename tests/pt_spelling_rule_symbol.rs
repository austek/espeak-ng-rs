// The rule symbol `W` matches only while a word is spelled letter by letter
// and more letters follow (C's `RULE_SPELLING` byte).  pt `?1 _) g (_W Ze`
// ("within an acronym") therefore gives `Ze` to an inner `g`, while a lone or
// last `g` takes `?1 _) g (_ ge`.
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
fn lone_and_last_g_is_hard() {
    check(
        "pt",
        &[
            ("g", "ɡˈe"),
            ("G", "ɡˈe"),
            ("b g", "bˈe ɡˈe"),
            ("a g b", "ɐ ɡˈe bˈe"),
            ("ng", "ˌɛnɡˈe"),
            ("lg", "ˌɛlɡˈe"),
            ("sg", "ˌɛsɡˈe"),
            ("rg", "ˌɛrrɡˈe"),
            ("rng", "ˌɛrrʲˌɛnɡˈe"),
            ("bng", "bˌeˌɛnɡˈe"),
        ],
    );
}

#[test]
fn inner_acronym_g_keeps_its_soft_name() {
    check("pt", &[("GG", "ʒˌeɡˈe"), ("xg", "ʃˌiʒɡˈe")]);
}
