// tests/spellword_abort.rs
//
// A letter no rule covers aborts the word (C's `FLAG_SPELLWORD`) unless it is
// the first letter and an accented one follows: the word is then read as
// separate one-letter words.  Expected output is from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn an_unknown_letter_spells_its_word_letter_by_letter() {
    let cases = [
        ("fr", "aʃb", "ˈa (en)ˈɛʃ(fr) bˈe"),
        ("fr", "ʃab", "(en)ˈɛʃ(fr) ˈa bˈe"),
        ("fr", "abʃ", "ˈa bˈe (en)ˈɛʃ(fr)"),
        ("fr", "éʃ", "ˌəaksɑ̃tɛɡˈy (en)ˈɛʃ(fr)"),
        ("fr", "bonjour ʃab merci", "bɔ̃ʒˈuʁ (en)ˈɛʃ(fr) a bˈe mɛʁsˈi"),
        ("fr", "aªb", "ˈa ˈa bˈe"),
        ("es", "aʃb", "ˈa (en)ˈɛʃ(es) bˈe"),
        ("es", "abʃ", "ˈa βˈe (en)ˈɛʃ(es)"),
        ("nl", "aʃb", "ˈaː (en)ˈɛʃ(nl) bˈeː"),
        ("nl", "ʃab", "(en)ˈɛʃ(nl) ˈaː bˈeː"),
        ("de", "aʃb", "ˈɑː (en)ˈɛʃ(de) bˈeː"),
        ("pl", "aʃb", "a (en)ˈɛʃ(pl) bˈɛ"),
        ("tr", "abʃ", "ˈa bˈe (en)ˈɛʃ(tr)"),
        ("tr", "aªb", "ˈa ˈa bˈe"),
    ];
    let mut failures = Vec::new();
    for (lang, text, expected) in cases {
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
fn a_leading_unknown_letter_before_a_non_ascii_letter_is_skipped() {
    let cases = [
        ("fr", "ʃé", "ˈe"),
        ("es", "ʃé", "ˈe"),
        ("nl", "ʃé", "ˈeː"),
        ("de", "ʃé", "ˈeː"),
        ("it", "ʃé", "ˈe"),
    ];
    for (lang, text, expected) in cases {
        let Ok(t) = Translator::new(lang, None) else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        assert_eq!(t.text_to_ipa(text).unwrap(), expected, "{lang} {text:?}");
    }
}
