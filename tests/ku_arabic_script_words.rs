// A word handed to another voice by its script, in which that voice's rules
// abandon a letter, is spelled letter by letter in the original voice (C's
// `FLAG_SPELLWORD` from the second translator).  Expected strings come from C
// espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

#[test]
fn arabic_word_with_an_unknown_letter_is_spelled_in_kurdish() {
    let Ok(t) = Translator::new("ku", None) else {
        eprintln!("[SKIP] no ku data");
        return;
    };
    let cases = [
        ("هە", " (en)ˈaɹəbɪk(ku)hˈaːʔ (en)ˈaɹəbɪk(ku)tipaʃɛʃdɛpˈendʒ"),
        (
            "بەر",
            " (en)ˈaɹəbɪk(ku)bˈaːʔ (en)ˈaɹəbɪk(ku)tipaʃɛʃdɛpˈendʒ (en)ˈaɹəbɪk(ku)rˈaʊːʔ",
        ),
        ("ڕ", "(en)ˈaɹəbɪk(ku)tipaʃɛʃnɛhpˈendʒ"),
        ("ژێ", " (en)ˈaɹəbɪk(ku)tipaʃɛʃnɛhhɛjʃt (en)ˈaɹəbɪk(ku)tipaʃɛʃdʒˈeɛ"),
        ("هو", "(ar)hˈuwa(ku)"),
        ("ەر", "(ar)r(ku)"),
        ("ڕوو", "(ar)ww(ku)"),
    ];
    let mut failures = Vec::new();
    for (text, expected) in cases {
        let got = t.text_to_ipa(text).unwrap();
        if got.trim_end() != expected {
            failures.push(format!("{text:?}: got {got:?}, C says {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
