// tests/symbol_table_removed.rs
//
// Symbols are read only as the language's own data and C's English fallback
// read them.  The port used to carry an invented per-language word table
// (`×` → "por", `÷` → "diviso", `±` → "mais menos", `©` → "copyright"); C
// speaks nothing for those in es/it/ru/de, and falls back to English for
// `±` in pt and tr.
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
fn multiplication_and_division_signs_are_silent_where_c_is() {
    check!("es", "×", "");
    check!("es", "÷", "");
    check!("it", "÷", "");
    check!("ru", "×", "");
    check!("tr", "×", "");
}

#[test]
fn plus_minus_sign_falls_back_to_english() {
    check!("pt", "±", "(en)plˈʌs ɔː mˈaɪnəs(pt-pt)");
    check!("tr", "±", "(en)plˈʌs ɔː mˈaɪnəs(tr)");
}

#[test]
fn degree_sign_is_silent_in_russian() {
    check!("ru", "20°", "dvˈɑtsʌtʲ");
}

#[test]
fn dropped_symbols_in_hebrew_and_ukrainian() {
    check!("he", "10÷2", "ˈeser ʃtˈajim");
    check!("he", "20°", "esrˈim");
    check!("uk", "2×3", "dβˈa trˈɪ");
}
