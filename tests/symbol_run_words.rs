// tests/symbol_run_words.rs
//
// C reads a run of adjacent symbols (`://`, `//`, `##`, `~~`) as one word: the
// list entry of the whole run, else the readings of its characters joined and
// stressed as a single word (`sl'aSslaS`, not `sl'aS sl'aS`).  A character the
// language cannot name takes the whole run to English.
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
fn english_joins_a_run_into_one_stressed_word() {
    check!("en", "https://x.org", "ˌeɪtʃtˌiːtˈiːpˌiːˈɛs kˈəʊlənslˌaʃslaʃ ˈɛks dˈɒt ˈɔːɡ");
    check!("en", "ftp://a", "ˌɛftˌiːpˈiː kˈəʊlənslˌaʃslaʃ ˈeɪ");
    check!("en", "a :// b", "ɐ kˈəʊlənslˌaʃslaʃ bˈiː");
    check!("en", "a // b", "ɐ slˈaʃslaʃ bˈiː");
    check!("en", "a :/ b", "ɐ kˈəʊlənslˌaʃ bˈiː");
    check!("en", "a ## b", "ɐ hˈaʃhaʃ bˈiː");
    check!("en", "x ~~ y", "ˈɛks tˈɪldətˌɪldə wˈaɪ");
}

#[test]
fn other_languages_join_the_names_they_have() {
    check!("es", "http://x", "ˌatʃetˌetˌepˈe βˌaraβˈara ˈɛkis");
    check!("es", "a // b", "a βˌaraβˈara βˈe");
    check!("fr", "a // b", "a bˈe");
    check!("de", "a :// b", "ˈɑː bˈeː");
    check!("ru", "a // b", "(en)ˈeɪ(ru) kʌsˈɑja tʃʲirtˈɑkʌsˈɑja tʃʲirtˈɑ (en)bˈiː(ru)");
    check!("cs", "a // b", "a lˈomenˌolomˌeno bˈeː");
    check!("pl", "x ~~ y", "ˈiks ˈiɡrɛk");
    check!("hu", "a :// b", "ˌɑ kˈɛtːøːʃpontpɛrpɛr bˈeː");
}

#[test]
fn a_symbol_the_language_cannot_name_takes_the_run_to_english() {
    check!("ku", "a // b", "a (en)slˈaʃslaʃ(ku) bˈe");
    check!("ku", "ftp://a", "fˌetˈɛpʰ (en)kˈəʊlənslˌaʃslaʃ(ku) ˈa");
    check!("ku", "x ~~ y", "xˈe (en)tˈɪldətˌɪldə(ku) jˈe");
}
