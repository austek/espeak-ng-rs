// tests/numbers_indic_scales.rs
//
// Number words that depend on what follows the count (Tamil `_<n>o` before a
// thousand or lakh, `_<n>M<k>x` for an exact scale, Telugu `_0C0` for an exact
// hundred, Malayalam `_<n>f` before a thousand) and a listed number word that
// wins over the composed reading (Marathi `100`).  Expected values come from
// the C espeak-ng master binary (f0fb11a8).

use espeak_ng::Translator;

fn check(lang: &str, rows: &[(&str, &str)]) {
    let Ok(t) = Translator::new_default(lang) else {
        eprintln!("[SKIP] no {lang} dict");
        return;
    };
    for (text, ipa) in rows {
        assert_eq!(t.text_to_ipa(text).unwrap(), *ipa, "{lang} {text:?}");
    }
}

#[test]
fn tamil_variants_before_scale_words() {
    check("ta", &[
        ("2024", "ˈiɹʌɳɖ ˌaːjiɹˌʌttʉ ˌiɹʉbˌʌttʉnˌaːnɡʉ"),
        ("12345", "pˈʌnniɹˌʌɳɖ aːjˌiɹʌttʉ mˈunnuːʈʈrˌi naːrpˌʌttʉˌaɪndʉ"),
        ("25000", "ˈiɹʉbˌʌttʉˌaɪnd aːjˌiɹəm"),
        ("100000", "ˈoɹʉlˌʌɖtʃəm"),
    ]);
}

#[test]
fn malayalam_feminine_forms_before_thousand() {
    check("ml", &[
        ("2024", "ɾˈəɳd ˈaːjirˌɐtti ˌiɾuvˌɐttinˌaːlɨ"),
        ("3000", "mˈuːv ˈaːjirəm"),
        ("25000", "ˈiɾuvɐttˈɐiːj ˈaːjirəm"),
    ]);
}

#[test]
fn telugu_exact_hundred_before_scale() {
    check("te", &[
        ("5000000000", "ˈaɪduʋˌandalˌu kotlu"),
        ("7000000000", "ʲˈeːɖuʋˌandalˌu kotlu"),
    ]);
}

#[test]
fn marathi_listed_hundred() {
    check("mr", &[("100", "ʃˈʌmbəɾ"), ("200", "dˈoːnʃˈeː")]);
}
