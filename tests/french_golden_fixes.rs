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

fn phoneme_cases(cases: &[(&str, &str)]) {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    for (text, want) in cases {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
            .args(["-q", "-x", "-v", "fr", "--", text])
            .env("ESPEAK_DATA_PATH", data)
            .output()
            .expect("run cli");
        let got = String::from_utf8_lossy(&out.stdout).trim().replace('\n', " ");
        assert_eq!(&got, want, "text {text:?}");
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

#[test]
fn listed_number_symbols_read_as_dictionary_words() {
    ipa_cases(&[
        ("½", "œ̃dəmˈi"),
        ("¼", "œ̃kˈaʁ"),
        ("¾", "tʁwakˈaʁ"),
        ("²", "pyisˈɑ̃s dˈø"),
        ("³", "pyisɑ̃s tʁwˈa"),
        ("½ tasse", "œ̃dəmˈi tˈas"),
        ("x²", "ˈiks pyisˈɑ̃s dˈø"),
        ("10 m²", "dˈis ˈɛm pyisˈɑ̃s dˈø"),
        ("2²", "dˈø pyisˈɑ̃s dˈø"),
    ]);
}

#[test]
fn spelled_i_before_a_vowel_is_a_glide_without_stress() {
    ipa_cases(&[
        ("cia", "sˌejˈa"),
        ("cio", "sˌejˈo"),
        ("pcmcia", "pˌesˌeˈɛmsˌejˈa"),
    ]);
    phoneme_cases(&[("cia", "s,ej'a"), ("cio", "s,ej'o")]);
}
