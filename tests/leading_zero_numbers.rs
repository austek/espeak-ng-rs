// tests/leading_zero_numbers.rs
//
// A number with a leading zero is a code, not a quantity (C `TranslateNumber`):
// a short one (≤3 digits) speaks its leading zeros then the number, a longer one
// is spoken digit-by-digit.  C joins the zeros into one word and the spelled
// digits of a four-digit code into one word, so no space separates them.
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn check(lang: &str, cases: &[(&str, &str)]) {
    let Some(t) = Translator::new(lang, None).ok() else {
        eprintln!("[SKIP] no {lang} data");
        return;
    };
    for (text, want) in cases {
        assert_eq!(t.text_to_ipa(text).unwrap(), *want, "{lang} {text}");
    }
}

#[test]
fn leading_zeros_join_into_one_word() {
    check(
        "en",
        &[
            ("007", "zˈiəɹəʊzˈiəɹəʊ sˈɛvən"),
            ("000", "zˈiəɹəʊzˈiəɹəʊ zˈiəɹəʊ"),
            ("05", "zˈiəɹəʊ fˈaɪv"),
            ("00", "zˈiəɹəʊ zˈiəɹəʊ"),
        ],
    );
    check("de", &[("007", "nˈʊlnˈʊl zˈiːbən")]);
    check("fr", &[("007", "zeʁozeʁo sˈɛt")]);
    check("ru", &[("007", "nˈoɭnˈoɭ sʲˈejm")]);
    check("ar", &[("007", "s̪-ˈifrs̪-ˈifr sˈabʕʕˌa"), ("05", "s̪-ˈifr xˈamsa")]);
}

#[test]
fn four_digit_code_is_one_word() {
    check(
        "en",
        &[
            ("0800", "zˈiəɹəʊˈeɪtzˈiəɹəʊzˈiəɹəʊ"),
            ("0012", "zˈiəɹəʊzˈiəɹəʊwˈɒntˈuː"),
            ("0000", "zˈiəɹəʊzˈiəɹəʊzˈiəɹəʊzˈiəɹəʊ"),
        ],
    );
    check(
        "ar",
        &[
            ("0800", "s̪-ˈifrθamˈaːnijˌas̪-ˈifrs̪-ˈifr"),
            ("0100", "s̪-ˈifrwˈaːħids̪-ˈifrs̪-ˈifr"),
        ],
    );
}

#[test]
fn longer_codes_and_plain_numbers_are_unchanged() {
    check(
        "en",
        &[
            ("00123", "zˈiəɹəʊzˈiəɹəʊ wˈɒntˈuːθɹˈiː"),
            ("01234", "zˈiəɹəʊwˈɒn tˈuːθɹˈiːfˈɔː"),
            ("42", "fˈɔːti tˈuː"),
            ("0", "zˈiəɹəʊ"),
            ("012", "zˈiəɹəʊ twˈɛlv"),
        ],
    );
}
