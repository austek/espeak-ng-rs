// tests/roman_numerals.rs
//
// English reads a valid Roman numeral of two to forty-nine that the dictionary
// does not list as "roman" plus the number, in lower or upper case, exactly as
// upstream's `TranslateRoman` does.  Expected values are from upstream master.
// Skips without en data.

use espeak_ng::Translator;

fn en() -> Option<Translator> {
    Translator::new("en", None).ok()
}

#[test]
fn valid_numerals_read_as_roman_plus_number() {
    let Some(t) = en() else { eprintln!("[SKIP] no en data"); return };
    let ipa = |s: &str| t.text_to_ipa(s).unwrap();
    assert_eq!(ipa("Chapter IV"), "tʃˈaptə ɹˌəʊmən fˈɔː");
    assert_eq!(ipa("World War II"), "wˈɜːld wˈɔː ɹˌəʊmən tˈuː");
    assert_eq!(ipa("Section VII"), "sˈɛkʃən ɹˌəʊmən sˈɛvən");
    assert_eq!(ipa("Louis XIV"), "lˈuːi ɹˌəʊmən fˈɔːtiːn");
    assert_eq!(ipa("vi"), "ɹˌəʊmən sˈɪks");
    assert_eq!(ipa("xxi"), "ɹˌəʊmən twˈɛnti wˈɒn");
}

#[test]
fn listed_capitalised_and_single_letter_words_are_left_alone() {
    let Some(t) = en() else { return };
    let ipa = |s: &str| t.text_to_ipa(s).unwrap();
    assert_eq!(ipa("mix"), "mˈɪks");
    assert_eq!(ipa("Iv"), "ˈɪv");
    assert_eq!(ipa("did"), "dˈɪd");
    assert_eq!(ipa("VI"), "vˌiːˈaɪ");
    assert_eq!(ipa("Section A"), "sˈɛkʃən ˈeɪ");
}
