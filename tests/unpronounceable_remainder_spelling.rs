// A word whose remainder after the spelled leading letters holds a letter the
// rules abandon is redone letter by letter; `nextVowel` conditions of the IPA
// pass see vowels across spelled letter names; Belarusian dictionary keys use
// the Latin transpose map. Expected strings are espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn abandoned_remainder_respells_the_whole_word() {
    if ipa("en", "1").is_none() {
        return;
    }
    assert_eq!(
        ipa("bg", "хлѣбъ").as_deref(),
        Some("xˈə lˈə bˈukvatʃˈetirˌiʃˈesttrˈi bˈə ˈerɡˌolɐm")
    );
}

#[test]
fn bulgarian_unstressed_a_before_stressed_vowel_keeps_its_quality() {
    if ipa("en", "1").is_none() {
        return;
    }
    assert_eq!(ipa("bg", "България").as_deref(), Some("bˌəɫɡarˈiɐ"));
    assert_eq!(ipa("bg", "ѫ").as_deref(), Some("bˈukvatʃˈetirˌiʃˈestbˈiː"));
}

#[test]
fn belarusian_letter_names_come_from_the_dictionary() {
    if ipa("en", "1").is_none() {
        return;
    }
    assert_eq!(ipa("be", "д").as_deref(), Some("dˈɛ"));
    assert_eq!(ipa("be", "в г").as_deref(), Some("vɛ ɣˈɛ"));
    assert_eq!(
        ipa("be", "дѣло").as_deref(),
        Some("dɛ lˈetəʈ͡ʂatˈɨrɨʂˈɛsʲt͡sʲtrˈɨ ɛl ɔ")
    );
}
