// A lengthener after a pause doubles the pause (C's `types_double` search
// also matches type 0), and repeated lengtheners collapse into one flag.
//
// Values from C espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn de() -> Option<Translator> {
    let t = Translator::new("de", Some(std::path::Path::new("espeak-ng-data"))).ok()?;
    t.text_to_ipa("a").ok()?;
    Some(t)
}

#[test]
fn german_highend_has_no_length_mark() {
    let Some(t) = de() else { return };
    assert_eq!(t.text_to_ipa("highend").unwrap(), "hˌaiˈɛnt");
}

#[test]
fn repeated_lengtheners_collapse() {
    let Some(t) = de() else { return };
    assert_eq!(t.text_to_ipa("[[ma::]]").unwrap(), "mˈaː");
    assert_eq!(t.text_to_ipa("[[m::a]]").unwrap(), "mːˈa");
}
