// Vulgar fractions that the language's list names (`pt_list`: `¼ u~Nku'a*@-tU`)
// are spoken from that entry, not rewritten to `1/4` and read as a "barra"
// fraction.
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
fn european_listed_fractions_use_their_entry() {
    check(
        "pt",
        &[
            ("¼", "ˌu\u{303}ŋkuˈaɾətʊ"),
            ("½", "mˈeɪʊ"),
            ("⅓", "ˌu\u{303}ŋtˈeɾəsʊ"),
            ("⅔", "dˌoɪʃtˈeɾəsʊʃ"),
            ("¾", "tɹˌeʃkwˈaɾətʊʃ"),
            ("3½", "tɹˈeʒ mˈeɪʊ"),
            ("um ¼ de litro", "u\u{303}ŋ ˌu\u{303}ŋkuˈaɾətʊ dɨ lˈitɹʊ"),
        ],
    );
}

#[test]
fn brazilian_listed_fractions_use_their_entry() {
    check(
        "pt-br",
        &[
            ("¼", "ˌu\u{303}ŋkuˈaɾətʊ"),
            ("½", "mˈeɪʊ"),
            ("3½", "trˈez mˈeɪʊ"),
        ],
    );
}

#[test]
fn italian_listed_fraction_uses_its_entry() {
    check("it", &[("¼", "ʊn kʊˈarto"), ("½", "ʊn mˈɛdzːo")]);
}
