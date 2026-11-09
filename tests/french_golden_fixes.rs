// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v fr`
// and `-q -x -v fr`.

use espeak_ng::Translator;

fn ipa_cases(cases: &[(&str, &str)]) {
    let Ok(t) = Translator::new("fr", Some(std::path::Path::new("espeak-ng-data"))) else {
        eprintln!("[SKIP] no fr data");
        return;
    };
    for (word, want) in cases {
        let got = t.text_to_ipa(word).unwrap_or_default();
        assert_eq!(&got, want, "word {word:?}");
    }
}

#[test]
fn abbrev_entries_spell_letters_with_stress() {
    ipa_cases(&[
        ("ratp", "ˌɛʁˌatˌepˈe"),
        ("abc", "ˌabˌesˈe"),
        ("USA", "ˌyˌɛsˈa"),
        ("avg", "ˌavˌeʒˈe"),
        ("riaa", "ˌɛʁˌjˌaˈa"),
        ("pda", "pˌedˌeˈa"),
        ("sap", "ˌɛsˌapˈe"),
        ("ac", "ˌasˈe"),
        ("tva", "tˌevˌeˈa"),
        ("nvda", "ˌɛnvˌedˌeˈa"),
        ("wma", "dubləvˌeˌɛmˈa"),
    ]);
}
