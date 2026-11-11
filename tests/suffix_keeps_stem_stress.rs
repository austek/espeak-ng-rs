// A suffix rule flagged `t` leaves the stem's stress alone: the stem is stressed
// first and the ending's phonemes are added afterwards.
//
// Values from C espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn tr(lang: &str) -> Option<Translator> {
    let t = Translator::new(lang, Some(std::path::Path::new("espeak-ng-data"))).ok()?;
    t.text_to_ipa("a").ok()?;
    Some(t)
}

fn check(lang: &str, cases: &[(&str, &str)]) {
    let Some(t) = tr(lang) else {
        eprintln!("[SKIP] no {lang} data");
        return;
    };
    for (word, ipa) in cases {
        assert_eq!(t.text_to_ipa(word).ok().as_deref(), Some(*ipa), "{lang} {word}");
    }
}

#[test]
fn polish_name_endings_keep_the_stem_stress() {
    check(
        "pl",
        &[
            ("alison", "ˈalisɔn"),
            ("malison", "mˈalisɔn"),
            ("edison", "ˈɛdʲisɔn"),
            ("eriksen", "ˈɛriksɛn"),
            ("wellington", "vˈɛlliŋktɔn"),
            ("heisenberg", "xɛˈisɛnbɛrɡ"),
        ],
    );
}

#[test]
fn polish_prefix_before_a_stem_with_such_an_ending() {
    check("pl", &[("mcalison", "mˈɛkˌalisɔn"), ("vicegregson", "witsɛɡrˈɛksɔn")]);
}

#[test]
fn spanish_mente_keeps_both_stresses() {
    check(
        "es",
        &[
            ("aprox", "ˌapɾoksimˈaðamˈente"),
            ("aproximadamente", "ˌapɾoksimˈaðamˈente"),
            ("rápidamente", "rˈapiðamˈente"),
            ("claramente", "klˈaɾamˈente"),
            ("felizmente", "felˈiθmˈente"),
        ],
    );
}

#[test]
fn catalan_ment_keeps_both_stresses() {
    check(
        "ca",
        &[
            ("exactament", "əɡzˈaktəmˈen"),
            ("ràpidament", "rˈapiðəmˈen"),
            ("l'exactament", "ləɡzˈaktəmˈen"),
        ],
    );
}
