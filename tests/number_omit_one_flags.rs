// Whether "one" is spoken before "hundred" / "thousand" follows each language's
// NUM_OMIT_1_HUNDRED and NUM_OMIT_1_THOUSAND bits (tr_languages.c), not the
// language family. Expected strings are espeak-ng master (f0fb11a8) output.

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
fn ukrainian_keeps_one_before_thousand() {
    check(
        "uk",
        &[
            ("1000", "odˈen tˈesjatʃa"),
            ("1100", "odˈen tˈesjatʃa stˈo"),
            ("1001", "odˈen tˈesjatʃa odˈen"),
        ],
    );
}

#[test]
fn belarusian_keeps_one_before_thousand() {
    check("be", &[("1000", "adna tˈɨsʲaʈ͡ʂa")]);
}

#[test]
fn catalan_omits_one_before_thousand() {
    check("ca", &[("1000", "mˈil")]);
}
