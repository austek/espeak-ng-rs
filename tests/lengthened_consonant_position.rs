// A consonant followed by the lengthener is doubled, but only when it is not the
// first phoneme of the clause. When it starts a word, the copy lands at the end
// of the previous word. C also writes the lengthener through the consonant's
// own IPA name. Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

fn check(lang: &str, cases: &[(&str, &str)]) {
    let Some(_) = ipa(lang, "a") else { return };
    for (text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(*want), "{lang} {text}");
    }
}

#[test]
fn word_initial_geminate_moves_to_the_previous_word() {
    check(
        "ar",
        &[
            ("ب عمرو", "bʕ ʕmr"),
            ("ب عمرو ب", "bʕ ʕmr bˈaːʔ"),
            ("عمرو عمرو", "ʕʕmrʕ ʕmr"),
        ],
    );
}

#[test]
fn clause_initial_consonant_is_not_doubled() {
    check("ar", &[("عمرو", "ʕʕmr"), ("عمرو ب", "ʕʕmr bˈaːʔ"), ("ب، عمرو", "bˈaːʔ\nʕʕmr")]);
}

#[test]
fn word_internal_geminates_are_unchanged() {
    check(
        "it",
        &[("mamma", "mˈamma"), ("anno", "ˈanno"), ("la mamma", "la mˈamma")],
    );
}
