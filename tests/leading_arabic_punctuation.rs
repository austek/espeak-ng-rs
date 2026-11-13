// An Arabic comma, semicolon or question mark before any letter of the clause
// does not end the clause: it is read as a word, by its name in the list.
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn leading_arabic_punctuation_is_spoken_in_farsi() {
    if ipa("en", "1").is_none() {
        return;
    }
    let cases = [
        ("fa", "،", "vˈirɡul"),
        ("fa", "؟", "ˌalɑmˌatesˈoɑl"),
        ("fa", "؛", "nˌoq1tevˈirɡul"),
        ("fa", "، سلام", "vˈirɡul salˈɑm"),
        ("fa", "؟ سلام", "ˌalɑmˌatesˈoɑl salˈɑm"),
        ("fa", "، ،", "vˈirɡul vˈirɡul"),
        ("fa", "، a", "vˈirɡul (en)ˈeɪ(fa)"),
        ("fa", "سلام ،", "salˈɑm"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}
