// tests/symbol_english_switch.rs
//
// C's `LookupLetter` (numbers.c, `next_byte == -1`): a character the language
// neither names nor lists as `_<char>` is looked up in English; when English
// has a bare entry the word is re-translated in English.  `\` ("backslash")
// therefore speaks in every language, though it is in no fixed symbol set.
//
// Expectations are from upstream espeak-ng master (f0fb11a8), `-q --ipa`.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    Translator::new(lang, None).ok().map(|t| t.text_to_ipa(text).unwrap())
}

macro_rules! check {
    ($lang:expr, $text:expr, $want:expr) => {
        match ipa($lang, $text) {
            Some(got) => assert_eq!(got, $want, "{} {:?}", $lang, $text),
            None => eprintln!("[SKIP] no {} data", $lang),
        }
    };
}

#[test]
fn backslash_is_spoken_through_english() {
    check!("ku", "a \\ b", "a (en)bˈakslaʃ(ku) bˈe");
    check!("sv", "a \\ b", "ˈɑː (en)bˈakslaʃ(sv) bˈeː");
    check!("ar", "a \\ b", "(en)ˈeɪ bˈakslaʃ bˈiː(ar)");
}

#[test]
fn punctuation_with_only_an_underscore_name_stays_silent() {
    check!("ku", "a [ b", "a bˈe");
    check!("ku", "a | b", "a bˈe");
}
