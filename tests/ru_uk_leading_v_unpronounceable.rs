// Russian and Ukrainian do not count a leading в when testing whether a word
// is unpronounceable (LOPT_UNPRONOUNCABLE = 0x432), so в plus three consonants
// is read as a word. Expected strings are espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn leading_v_with_three_consonants_is_a_word() {
    if ipa("en", "1").is_none() {
        return;
    }
    let cases = [
        ("ru", "взгляд", "vzɡɭʲˈɑt"),
        ("ru", "встреча", "fstrʲˈetʃʲʌ"),
        ("ru", "Встреча", "fstrʲˈetʃʲʌ"),
        ("ru", "вздрогнуть", "vzdrˈoɡnutʲ"),
        ("ru", "Вскрытие", "fskrˈytʲijɪ"),
        ("ru", "Всплеск", "fspɭʲˈesk"),
        ("ru", "Взбрело", "vzbrʲiɭˈo"),
        ("ru", "Встряска", "fstrʲˈɑska"),
        ("ru", "Встретимся", "fstrʲˈetʲimsʲʌ"),
        ("uk", "вздрогнути", "βzdrohnˈut̪ɪ"),
        ("uk", "Встромити", "fst̪romˈɪt̪ɪ"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}

#[test]
fn ordinary_words_unchanged() {
    if ipa("en", "1").is_none() {
        return;
    }
    let cases = [
        ("ru", "страна", "stranˈɑ"),
        ("ru", "вторник", "ftˈornʲik"),
        ("ru", "взрыв", "vzrˈyf"),
        ("uk", "вдруге", "βdrˈuhɛ"),
        ("uk", "страх", "st̪rˈax"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}
