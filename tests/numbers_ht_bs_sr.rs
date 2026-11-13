// tests/numbers_ht_bs_sr.rs
//
// C-flag driven number grammar: Haitian Creole (`NUM_VIGESIMAL`,
// `NUM_OMIT_1_HUNDRED`) and Bosnian/Serbian (`NUM_OMIT_1_HUNDRED`, Croatian
// thousand declension).  Expected values come from the C espeak-ng master
// binary (f0fb11a8).

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
fn haitian_vigesimal_and_bare_hundred() {
    check("ht", &[
        ("99", "diznˈef"),
        ("91", "ˈonz"),
        ("101", "sɑ̃ yoˈun"),
        ("199", "sɑ̃ diznˈef"),
        ("1234", "mil desɑ̃ katˈoz"),
        ("2500", "de mil senksˈɑ̃"),
    ]);
}

#[test]
fn bosnian_hundred_and_thousands() {
    check("bs", &[
        ("100", "stˈo"),
        ("101", "stˈo jˈedæn"),
        ("2000", "dvˈije hˈiʎade"),
        ("5000", "pˈɛt hˈiʎadæ"),
        ("21000", "dvˌadɛsɛtjˈedæn hˈiʎadʊ"),
    ]);
}

#[test]
fn serbian_hundred_and_thousands() {
    check("sr", &[
        ("100", "stˈɔ"),
        ("2024", "dvˈɛ hˈiʎˌɐde dvˌadɛsɛttʃˈɛtɪrɪ"),
        ("3000", "trˈi hˈiʎˌɐde"),
        ("5000", "pˈɛt hˈiʎˌɐdɐ"),
    ]);
}
