// The empty clause C reports for a closer after the last mark comes from the
// engine, so every front end shares it.  Values from upstream espeak-ng master
// (f0fb11a8): `-q -x -v en` and `-q --ipa -v en` print one empty line more.

use espeak_ng::{phoneme_text::format_mnemonics, PhonemeData, Translator};
use std::path::Path;

fn data() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data"))
}

fn mnemonics(text: &str, sep: Option<char>) -> String {
    let t = Translator::new("en", Some(data())).expect("translator");
    let codes = t.translate_to_clause_codes(text).expect("codes");
    let mut phdata = PhonemeData::load(data()).expect("phonemes");
    espeak_ng::translate::select_phoneme_table(&mut phdata, data(), "en").expect("table");
    format_mnemonics(&codes, &phdata, sep, None, data(), "en")
}

#[test]
fn mnemonics_keep_the_closing_clause() {
    assert_eq!(mnemonics("Come here!\"", None), "k'Vm h'i@3\n");
    assert_eq!(mnemonics("Come here!\"", Some('_')), "k_'V_m h_'i@3\n");
    assert_eq!(mnemonics("Come here!", None), "k'Vm h'i@3");
    assert_eq!(mnemonics("A 5\" pipe", None).lines().count(), 1);
}

#[test]
fn ipa_keeps_the_closing_clause() {
    let t = Translator::new("en", Some(data())).expect("translator");
    assert_eq!(t.text_to_ipa_clauses("Hello! \"").expect("ipa"), "həlˈəʊ\n");
    assert_eq!(t.text_to_ipa_clauses("Hello!").expect("ipa"), "həlˈəʊ");
}
