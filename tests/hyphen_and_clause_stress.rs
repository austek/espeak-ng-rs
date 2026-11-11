// A hyphen is a word break that rule contexts can match (`-) t (-`), a word
// switched to another voice after a hyphen stays glued to the previous one, the
// previous word's list flags (`$verbf`) choose the form of the next word, a rule
// pre-context can reach into the previous word (`por_) que`), and a foreign
// primary stress counts for the clause.  Expected strings are C espeak-ng
// master output.
use espeak_ng::Translator;

fn check(cases: &[(&str, &str, &str)]) {
    for (lang, text, expected) in cases {
        let Ok(t) = Translator::new_default(lang) else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        assert_eq!(&t.text_to_ipa(text).unwrap(), expected, "{lang} {text}");
    }
}

#[test]
fn a_letter_between_hyphens_matches_the_hyphen_context() {
    check(&[
        ("fr", "y a-t-il quelqu'un", "i atil kɛlkˈœ̃"),
        ("fr", "va-t-en", "vˈatˈɑ̃"),
        ("fr", "dit-il", "dˈitˈil"),
        ("fr", "est-ce que tu viens", "ɛs kə ty vjˈɛ̃"),
    ]);
}

#[test]
fn a_switched_word_after_a_hyphen_is_kept_and_glued() {
    check(&[
        ("fr", "week-end", "(en)wˈiːkˈɛnd(fr)"),
        ("de", "E-Mail", "ˈeː(en)mˈeɪl(de)"),
    ]);
}

#[test]
fn a_foreign_primary_stress_keeps_the_native_word_unstressed() {
    check(&[
        ("fr", "un week", "œ̃ (en)wˈiːk(fr)"),
        ("fr", "un week end", "œ̃ (en)wˈiːk ˈɛnd(fr)"),
    ]);
}

#[test]
fn the_previous_words_list_flags_choose_the_verb_form() {
    check(&[
        ("fr", "nous sommes allés", "nu sɔmz alˈe"),
        ("fr", "ils ont vu", "ilz ɔ̃ vˈy"),
    ]);
}

#[test]
fn a_rule_pre_context_reaches_the_previous_word() {
    check(&[
        ("pt-br", "por que", "por kˈe"),
        ("pt-br", "por que não", "por ke nˈɐ̃ʊ̃"),
    ]);
}
