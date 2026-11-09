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
fn symbols_named_only_with_an_underscore_are_silent_in_running_text() {
    ipa_cases(&[
        ("1/2", "ˈœ̃ dˈø"),
        ("3/4", "tʁwˈa kˈatʁ"),
        ("a / b", "a bˈe"),
        ("km/h", "kilomˈɛtʁ ˈaʃ"),
        ("12/05/2024", "dˈuz zeʁo sˈɛ̃k dø mil vɛ̃tkˈatʁ"),
        ("a # b", "a bˈe"),
        ("a = b", "a bˈe"),
        ("a § b", "a bˈe"),
        ("3+4=7", "tʁwˈa plˈys kˈatʁ sˈɛt"),
        ("user@example.com", "yzˈe aʁobˈaz ɛɡzˈɑ̃pl kˈɔm"),
        ("www.exemple.fr", "dubləvˌedubləvˌedubləvˈe ɛɡzˈɑ̃pl ˌɛfˈɛʁ"),
    ]);
}

#[test]
fn number_dot_word_reads_the_point_unstressed() {
    ipa_cases(&[
        ("1.er", "œ̃ pwɛ̃ ˈe"),
        ("2.e", "dø pwɛ̃ ˈə"),
        ("3.e", "tʁwa pwɛ̃ ˈə"),
        ("1.x", "œ̃ pwɛ̃ ˈiks"),
        ("1.ab", "œ̃ pwɛ̃ ˈab"),
        ("5.ème", "sɛ̃k pwɛ̃ jˈɛm"),
        ("2.er", "dø pwɛ̃ ˈe"),
        ("1. er", "ˈœ̃ ˈe"),
    ]);
}

#[test]
fn clause_without_a_primary_stress_puts_the_tonic_on_the_last_undiminished_syllable() {
    ipa_cases(&[
        ("et ou", "ˈe u"),
        ("et/ou", "ˈe u"),
        ("de du", "dˈə- dy-"),
        ("je ne", "ʒˈə- nə-"),
        ("de de de", "də- də- dˈə-"),
        ("de de de de", "də- də- dˈə- də-"),
        ("de de de de de", "də- də- də- də- dˈə-"),
        ("de du, de", "dˈə- dy-\ndˈə-"),
        ("tu je", "tˈy ʒə-"),
        ("je tu", "ʒə- tˈy"),
    ]);
}

#[test]
fn word_equal_to_its_prefix_rule_keeps_the_prefix_unstressed() {
    ipa_cases(&[("cyber", "sˈibɛʁ")]);
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
