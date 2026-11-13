// A Latin word glued to a number is read by English with the digit before it,
// so the rule `1_) st` keeps the `st` of `1st` silent in a voice that hands
// Latin words to English. Expected strings are espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn ordinal_suffix_after_digit_is_silent_in_switching_voices() {
    if ipa("en", "1").is_none() {
        return;
    }
    let cases = [
        ("ta", "1st", "ˈonrʉ"),
        ("ta", "21st", "ˈiɹʉbˌʌttʉˌonrʉ"),
        ("ml", "1st", "ˈonnɨ"),
        ("te", "1st", "ˈokaʈi"),
        ("mr", "1st", "ˈeːk"),
        ("pa", "1st", "ˈɪkː"),
        ("or", "1st", "ˈekɔ"),
        ("bpy", "1st", "ˈammaː"),
        ("hi", "1st", "ˈeːk"),
        ("mn", "1st", "nˈeɡ"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}
