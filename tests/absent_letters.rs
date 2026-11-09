// tests/absent_letters.rs
//
// A letter the voice has no name for is looked up in English (`ß` is "sharp
// s"), or read as "letter" plus the digits of its code point.  Super- and
// subscript letters are read as their base letter.  Expected output is from C
// espeak-ng master.

use espeak_ng::Translator;

fn check(cases: &[(&str, &str, &str)]) {
    let mut failures = Vec::new();
    for &(lang, text, expected) in cases {
        let Ok(t) = Translator::new(lang, None) else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let got = t.text_to_ipa(text).unwrap();
        if got != expected {
            failures.push(format!("{lang} {text:?}: got {got:?}, C says {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn lone_letters_take_the_english_name() {
    check(&[
        ("fr", "ß", "(en)ʃˌɑːpˈɛs(fr)"),
        ("fr", "ŋ", "(en)ˈɛŋɡ(fr)"),
        ("fr", "ɐ", "(en)tˈɜːndˈeɪ(fr)"),
        ("fr", "ʃ", "(en)ˈɛʃ(fr)"),
        ("es", "ß", "(en)ʃˌɑːpˈɛs(es)"),
        ("es", "ŋ", "(en)ˈɛŋɡ(es)"),
        ("nl", "ʃ", "(en)ˈɛʃ(nl)"),
        ("pl", "ɐ", "(en)tˈɜːndˈeɪ(pl)"),
        ("sv", "ß", "(en)ʃˌɑːpˈɛs(sv)"),
        ("de", "ŋ", "(en)ˈɛŋɡ(de)"),
        ("tr", "æ", "(en)lˌɪɡɐtʃəeɪˈiː(tr)"),
        ("tr", "œ", "(en)lˌɪɡɐtʃəəʊˈiː(tr)"),
        ("tr", "ø", "(en)ˌəʊstɹˈəʊk(tr)"),
        ("tr", "ł", "(en)ˌɛlstɹˈəʊk(tr)"),
        ("ru", "ɐ", "(en)tˈɜːndˈeɪ(ru)"),
        ("fr", "un ŋ deux", "œ̃ (en)ˈɛŋɡ(fr) dˈø"),
        ("fr", "a ß b", "a (en)ʃˌɑːpˈɛs(fr) bˈe"),
        ("nl", "x ɐ ,", "ˈɪks (en)tˈɜːndˈeɪ(nl)"),
    ]);
}

#[test]
fn a_letter_nobody_names_is_read_as_its_code_point() {
    check(&[
        ("fr", "ı", "lˈɛtʁˈœ̃tʁwˈaˈœ̃"),
        ("fr", "ᵉ", "lˈɛtʁˈœ̃dˈekˈatʁnˈœf"),
        ("es", "ı", "sˈimboloˈunotɾˈesˈuno"),
        ("nl", "ı", "lˈɛtərˈeːndrˈiˈeːn"),
        ("pl", "ı", "litɛrajˈɛdɛntʃˈɨjˈɛdɛn"),
        ("de", "ı", "bˈuːxʃtɑːbəˈaɪnsdɾˈaɪˈaɪns"),
        ("sv", "ı", "bˈuːkstavənˈɛttrˈeːˈɛt"),
        ("pt", "ı", "lˈetɹɐˈumtɹˈeʃˈum"),
        ("en", "ı", "lˌɛtəwˈɒnθɹˈiːwˈɒn"),
        ("en", "ƒ", "lˌɛtəwˈɒnnˈaɪntˈuː"),
    ]);
}

#[test]
fn superscript_letters_are_read_as_their_base_letter() {
    check(&[
        ("fr", "ˢ", "ˈɛs"),
        ("fr", "ʳ", "ˈɛʁ"),
        ("fr", "ª", "ˈa"),
        ("es", "º", "ˈo"),
        ("nl", "ª", "ˈaː"),
        ("tr", "ˢ", "sˈɛ"),
        ("sv", "ª", "ˈɑː"),
        ("en", "ˢ", "ˈɛs"),
        ("en", "ª", "ˈeɪ"),
        ("ru", "ˢ", "(en)ˈɛs(ru)"),
        ("ru", "ª", "(en)ˈeɪ(ru)"),
    ]);
}
