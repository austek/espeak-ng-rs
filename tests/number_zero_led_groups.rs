// A number whose first digit group starts with zero and is followed by
// thousands groups (`0.007`, `00,007`) reads the first group as a word (its
// leading zeros spoken, then the value, "zero" for an all-zero group) and then
// names the scale, as C's `TranslateNumber` does. Expected strings are
// espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

fn check(lang: &str, cases: &[(&str, &str)]) {
    let Some(_) = ipa(lang, "1") else { return };
    for (text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(*want), "{lang} {text}");
    }
}

#[test]
fn german_zero_led_groups() {
    check(
        "de",
        &[
            ("0.007", "nˈʊl tˈaʊzənt zˈiːbən"),
            ("0.012", "nˈʊl tˈaʊzənt tsvˈœlf"),
            ("00.007", "nˈʊl nˈʊl tˈaʊzənt zˈiːbən"),
            ("0.000.007", "nˈʊl mɪljˈoːnən  zˈiːbən"),
        ],
    );
}

#[test]
fn english_zero_led_groups() {
    check(
        "en",
        &[
            ("0,007", "zˈiəɹəʊ θˈaʊzənd ən sˈɛvən"),
            ("0,012", "zˈiəɹəʊ θˈaʊzənd ən twˈɛlv"),
            ("00,007", "zˈiəɹəʊ zˈiəɹəʊ θˈaʊzənd ən sˈɛvən"),
            ("0,000,007", "zˈiəɹəʊ mˈɪliən  ən sˈɛvən"),
        ],
    );
}

#[test]
fn other_languages_zero_led_groups() {
    check("es", &[("0.007", "θˈeɾo mˈil sjˈete")]);
    check("fr", &[("0 007", "zeʁo mˈil sˈɛt")]);
    check("ru", &[("0.007", "nˈoɭ tˈysʲitʃʲ sʲˈejm")]);
    check("nl", &[("0.007", "nˈɵl dˌœyzɛnt zˈeːvən")]);
    check("it", &[("0.007", "dzˈɛɾo mˈiːla sˈɛtːe")]);
}
