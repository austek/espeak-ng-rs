// A consonant followed by the syllabic marker `-` (Arabic `A-`, the pharyngeal
// ʕ in "arbaʕuːn") counts as a syllable: it takes its own stress mark in front of
// the consonant, and C prints the consonant's name a second time after it.
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(text: &str) -> Option<String> {
    let t = Translator::new_default("ar").ok()?;
    t.text_to_ipa(text).ok()
}

fn check(cases: &[(&str, &str)]) {
    let Some(_) = ipa("1") else { return };
    for (text, want) in cases {
        assert_eq!(ipa(text).as_deref(), Some(*want), "ar {text}");
    }
}

#[test]
fn syllabic_pharyngeal_is_doubled() {
    check(&[
        ("4", "ʔˈarbaʕʕˌa"),
        ("7", "sˈabʕʕˌa"),
        ("9", "tˈisʕʕˌa"),
        ("40", "ʔˌarbaʕʕˈuːn"),
        ("70", "sˌabʕʕˈuːn"),
        ("90", "tˌisʕʕˈuːn"),
        ("307", "θalˈaːθumˌiʔawa sˈabʕʕˌa"),
    ]);
}

#[test]
fn syllabic_pharyngeal_takes_its_own_stress() {
    check(&[
        ("10", "ˈʕʕˈaʃaɹˌa"),
        ("12", "ʔˈiθnˈaː ʕʕˌaʃar"),
        ("13", "θalˈaːθatˌa ʕʕˌaʃar"),
        ("14", "ʔarbˈaʕʕˌata ʕʕˌaʃar"),
    ]);
}

#[test]
fn unmarked_stress_carries_to_the_next_syllable() {
    check(&[
        ("ع", "ˈʕʕˈaːjn"),
        ("ب ع", "b ˈʕʕˈaːjn"),
        ("10 ع", "ˈʕʕˈaʃaɹˌa ˈʕʕˈaːjn"),
        ("عين", "ʕˈajn"),
    ]);
}

#[test]
fn compound_with_two_syllabic_consonants() {
    check(&[
        ("44", "ʔˈarbaʕʕˌawaʔˌarbaʕʕˈuːn"),
        ("74", "ʔˈarbaʕʕˌawasˌabʕʕˈuːn"),
        ("77", "sˈabʕʕˌawasˌabʕʕˈuːn"),
        ("79", "tˈisʕʕˌawasˌabʕʕˈuːn"),
    ]);
}

#[test]
fn numbers_without_a_syllabic_consonant_are_unchanged() {
    check(&[
        ("0", "s̪-ˈifr"),
        ("3", "θalˈaːθa"),
        ("8", "θamˈaːnijˌa"),
        ("30", "θˌalaːθˈuːn"),
        ("100", "mˈiʔa"),
    ]);
}
