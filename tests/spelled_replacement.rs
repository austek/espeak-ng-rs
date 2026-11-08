// tests/spelled_replacement.rs
//
// A word spelled letter by letter is spelled from the text *after* the rules
// file's `.replace` table has run.  English replaces `æ` by `ae`, and `ae` is
// an `$abbrev` entry, so `æ` is the names of `a` and `e`; the port used to
// look up the name of `æ` itself.  Expected output is from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn spelled_word_uses_the_replaced_text() {
    let t = Translator::new("en", None).unwrap();
    assert_eq!(t.text_to_ipa("æ").unwrap(), "ˌeɪˈiː");
    assert_eq!(t.text_to_ipa("Æ").unwrap(), "ˌeɪˈiː");
}
