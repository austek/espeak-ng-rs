// A word whose remainder after the spelled leading letters holds a letter the
// rules abandon is redone letter by letter; `nextVowel` conditions of the IPA
// pass see vowels across spelled letter names; Belarusian dictionary keys use
// the Latin transpose map. Expected strings are espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> String {
    Translator::new_default(lang)
        .unwrap_or_else(|e| panic!("voice data for {lang} is missing: {e}"))
        .text_to_ipa(text)
        .unwrap_or_else(|e| panic!("{lang} {text:?}: {e}"))
}

#[test]
fn abandoned_remainder_respells_the_whole_word() {
    assert_eq!(
        ipa("bg", "хлѣбъ"),
        "xˈə lˈə bˈukvatʃˈetirˌiʃˈesttrˈi bˈə ˈerɡˌolɐm"
    );
}

#[test]
fn bulgarian_unstressed_a_before_stressed_vowel_keeps_its_quality() {
    assert_eq!(ipa("bg", "България"), "bˌəɫɡarˈiɐ");
    assert_eq!(ipa("bg", "ѫ"), "bˈukvatʃˈetirˌiʃˈestbˈiː");
}

#[test]
fn belarusian_letter_names_come_from_the_dictionary() {
    assert_eq!(ipa("be", "д"), "dˈɛ");
    assert_eq!(ipa("be", "в г"), "vɛ ɣˈɛ");
    assert_eq!(
        ipa("be", "дѣло"),
        "dɛ lˈetəʈ͡ʂatˈɨrɨʂˈɛsʲt͡sʲtrˈɨ ɛl ɔ"
    );
}
