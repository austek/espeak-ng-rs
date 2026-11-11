// A word with a prefix and a suffix is looked up as prefix + stem before the
// stem is translated alone, and a phoneme the program deletes is invisible to
// the programs of the phonemes after it.
//
// Values from C espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn de() -> Option<Translator> {
    let t = Translator::new("de", Some(std::path::Path::new("espeak-ng-data"))).ok()?;
    t.text_to_ipa("a").ok()?;
    Some(t)
}

#[test]
fn listed_stem_with_prefix_and_suffix() {
    let Some(t) = de() else { return };
    assert_eq!(t.text_to_ipa("unsereren").unwrap(), "ˈʊnzɜərən");
}

#[test]
fn rhotic_after_a_deleted_rhotic() {
    let Some(t) = de() else { return };
    assert_eq!(t.text_to_ipa("Österreich").unwrap(), "ˈøːstɜrˌaɪç");
    assert_eq!(t.text_to_ipa("Kösterreich").unwrap(), "kˈøːstɜrˌaɪç");
}
