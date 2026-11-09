// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v en`.

use espeak_ng::Translator;

fn check(cases: &[(&str, &str)]) {
    let Ok(t) = Translator::new("en", Some(std::path::Path::new("espeak-ng-data"))) else {
        eprintln!("[SKIP] no en data");
        return;
    };
    for (word, want) in cases {
        let got = t.text_to_ipa(word).unwrap_or_default();
        assert_eq!(&got, want, "word {word:?}");
    }
}

#[test]
fn magic_e_rule_silences_the_next_e() {
    check(&[
        ("dofeet", "dˈəʊfɪt"),
        ("fubypeep", "fˈʌbaɪpˌɛp"),
        ("dopeep", "dˈəʊpɛp"),
        ("deetroneey", "dˈiːtɹəʊni"),
        ("poseag", "pˈəʊsaɡ"),
    ]);
}
