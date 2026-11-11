// The test for an unpronounceable word runs the rules with the next word in
// view, so `mc` before a word that starts with a consonant is `m@k`, not letters.
//
// Values from C espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn en() -> Option<Translator> {
    let t = Translator::new("en", Some(std::path::Path::new("espeak-ng-data"))).ok()?;
    t.text_to_ipa("a").ok()?;
    Some(t)
}

#[test]
fn mc_before_a_consonant_word() {
    let Some(t) = en() else { return };
    let want = "mək dˈɒnəld";
    assert_eq!(t.text_to_ipa("Mc Donald").unwrap(), want);
    assert_eq!(t.text_to_ipa("McDonald").unwrap(), want);
}

#[test]
fn mc_alone_is_spelled() {
    let Some(t) = en() else { return };
    assert_eq!(t.text_to_ipa("Mc").unwrap(), "ˌɛmsˈiː");
}
