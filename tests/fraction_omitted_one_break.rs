// A decimal fraction read as one number opens with a word break when the "one"
// before its thousand is omitted (Romanian "o mie"): C's `LookupNum3` writes
// `digits, break, thousand` with the digits empty. Expected strings are
// espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn romanian_fraction_with_omitted_one() {
    let Some(_) = ipa("ro", "1") else { return };
    assert_eq!(
        ipa("ro", "3,1416").as_deref(),
        Some("trˈeɪ vˈirɡulə ˌomˈie pˈatrusˈute ʃˈaɪsprezˌetʃe")
    );
}

#[test]
fn aromanian_fraction_with_omitted_one() {
    let Some(_) = ipa("rup", "1") else { return };
    assert_eq!(
        ipa("rup", "3,1416").as_deref(),
        Some("trˈeɪ virgˈˈilə ˈunɨ ɲˈiʎe pˈatrusˈuti ʃˌasprədzˈatsi")
    );
}
