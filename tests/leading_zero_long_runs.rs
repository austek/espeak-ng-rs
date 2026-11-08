// C joins the digit words of a run longer than 32 digits with no spaces and cuts
// it at the word buffer. Expected strings come from C espeak-ng master (f0fb11a).

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new(lang, None).ok()?;
    Some(t.text_to_ipa(text).unwrap())
}

macro_rules! need {
    ($lang:expr, $text:expr) => {
        match ipa($lang, $text) {
            Some(s) => s,
            None => {
                eprintln!("[SKIP] no {} data", $lang);
                return;
            }
        }
    };
}

#[test]
fn english_zero_and_thirty_two_digits() {
    assert_eq!(
        need!("en", "030741852963074185296307418529630"),
        "zˈiəɹəʊθɹˈiːzˈiəɹəʊsˈɛvənfˈɔːwˈɒnˈeɪtfˈaɪvtˈuːnˈaɪnsˈɪksθɹˈiːzˈiəɹəʊsˈɛvənfˈɔːwˈɒnˈeɪtfˈaɪvtˈuːnˈaɪnsˈɪksθɹˈiːzˈiəɹəʊsˈɛvənfˈɔːwˈɒnˈeɪtfˈaɪvtˈuːnˈaɪnsˈɪksθɹˈiːzˈiəɹəʊ"
    );
}

#[test]
fn english_zero_and_thirty_nine_nines() {
    assert_eq!(
        need!("en", "0999999999999999999999999999999999999999"),
        format!("zˈiəɹəʊ{}", "nˈaɪn".repeat(39))
    );
}

#[test]
fn english_hundred_digits_starting_with_zero() {
    assert_eq!(
        need!("en", "0307418529630741852963074185296307418529630741852963074185296307418529630741852963074185296307418529"),
        "zˈiəɹəʊθɹˈiːzˈiəɹəʊsˈɛvənfˈɔːwˈɒnˈeɪtfˈaɪvtˈuːnˈaɪnsˈɪksθɹˈiːzˈiəɹəʊsˈɛvənfˈɔːwˈɒnˈeɪtfˈaɪvtˈuːnˈaɪnsˈɪksθɹˈiːzˈiəɹəʊsˈɛvənfˈɔːwˈɒnˈeɪtfˈaɪvtˈuːnˈaɪnsˈɪksθɹˈiːzˈiəɹəʊsˈɛvənfˈɔːwˈɒnˈeɪtfˈaɪvtˈuːnˈaɪnsˈɪk"
    );
}

#[test]
fn german_zero_and_thirty_nine_digits() {
    assert_eq!(
        need!("de", "0307418529630741852963074185296307418529"),
        "nˈʊldɾˈaɪnˈʊlzˈiːbənfˈiːɾˈaɪnsˈaxtfˈʏnftsvˈaɪnˈɔønzˈɛksdɾˈaɪnˈʊlzˈiːbənfˈiːɾˈaɪnsˈaxtfˈʏnftsvˈaɪnˈɔønzˈɛksdɾˈaɪnˈʊlzˈiːbənfˈiːɾˈaɪnsˈaxtfˈʏnftsvˈaɪnˈɔønzˈɛksdɾˈaɪnˈʊlzˈiːbənfˈiːɾˈaɪnsˈaxtfˈʏnf"
    );
}

#[test]
fn kurdish_zero_and_thirty_nine_digits() {
    assert_eq!(
        need!("ku", "0307418529630741852963074185296307418529"),
        "sˈɪfrsˈesˈɪfrhˈɛfttʃˈarjˈɛkhˈɛjʃtpˈendʒdˈʊnˈɛhʃˈɛʃsˈesˈɪfrhˈɛfttʃˈarjˈɛkhˈɛjʃtpˈendʒdˈʊnˈɛhʃˈɛʃsˈesˈɪfrhˈɛfttʃˈarjˈɛkhˈɛjʃtpˈendʒdˈʊnˈɛhʃˈɛʃsˈesˈɪfrhˈɛfttʃˈarjˈɛkhˈɛjʃtpˈendʒdˈʊnˈɛh"
    );
}
