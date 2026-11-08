// tests/accented_letters.rs
//
// A letter without a name of its own is spoken as its base letter plus the
// name of its accent, built from C's `LookupAccentedLetter` tables: `é` is
// "e acute", `æ` is "a e ligature", `ʈ` is "t retroflex".  This applies to
// entries flagged `$accent` in the dictionary and to any lone letter the
// rules do not cover.  Expected output is from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn accented_letters_match_c() {
    let cases = [
        ("de", "é", "ˌeːaksˌɑ̃tɛɡˈyː"),
        ("de", "ç", "tsˌeːseːdˈiːj"),
        ("de", "æ", "lˌiːɡatˈuːɾɑːˈeː"),
        ("de", "à", "ˌɑːɡɾˈɑːvɪs"),
        ("de", "ĳ", "lˌiːɡatˈuːɾiːjˈɔt"),
        ("en", "ê", "ˌiːsˈɜːkəmflˌɛks"),
        ("en", "ñ", "ˌɛntˈɪldə"),
        ("en", "ʈ", "tˌiːɹˈɛtɹəflˌɛks"),
        ("fr", "è", "ˌəaksɑ̃ɡʁˈav"),
        ("fr", "ç", "sˌesedˈij"),
        ("fr", "ŕ", "ˌɛʁaksɑ̃tɛɡˈy"),
        ("es", "ã", "ˌatˈilðe"),
        ("es", "ø", "ˌobˈara"),
        ("pt", "à", "ˌɐɡɹˈavi"),
        ("it", "ô", "ˌɔːtʃirkonflˈesso"),
        ("pl", "ä", "ˌaˈumlawt"),
        ("nl", "á", "ˌaːˈaːixˌy"),
        ("af", "ê", "kˈɐpiˈiə"),
    ];
    let mut failures = Vec::new();
    for (lang, text, expected) in cases {
        let Ok(t) = Translator::new(lang, None) else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let got = t.text_to_ipa(text).unwrap();
        if got != expected {
            failures.push(format!("{lang} {text:?}: got {got:?}, C says {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
