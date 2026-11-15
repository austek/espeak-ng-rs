// Comma-grouped digits in the lakh languages (`1,23,45,678`) are one number:
// C removes the separators and breaks the digits at lakh and crore. A final
// group of two digits stays a number of its own. Expected strings are
// espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

fn check(lang: &str, cases: &[(&str, &str)]) {
    let Some(_) = ipa(lang, "1") else { return };
    for (text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(*want), "{lang} {text}");
    }
}

#[test]
fn hindi_lakh_groups() {
    check(
        "hi",
        &[
            ("12,34,567", "bˈaːɾəh lˈaːkʰ cˈɔ̃tis hˈʌɟaːɾ pˈa\u{303}csˈɔː sˈʌr.səʈʰ"),
            ("1,23,456", "ˈeːk lˈaːkʰ tˈeːis hˈʌɟaːɾ cˈaːrsˈɔː cʰˈʌpːən"),
            ("1,23,45", "ˈeːksˈɔː tˈeːis paɪntˈaːlis"),
            (
                "1,23,45,678",
                "ˈeːk kəɾˈoːr. tˈeːis lˈaːkʰ paɪntˈaːlis hˈʌɟaːɾ cʰˈʌhsˈɔː ʌthˈʌtːəɾ",
            ),
            ("3,00,000", "tˈiːn lˈaːkʰ"),
        ],
    );
}

#[test]
fn bengali_and_urdu_lakh_groups() {
    check(
        "bn",
        &[("1,23,45,678", "ˈæk kˈoʈi tˈejʃ lˈakʰ pˈɔ̃jtallˈiʃ hˈajaɾ tʃʰˈɔʃˈo ˈaʈattˈɔɾ")],
    );
    check("ur", &[("1,23,456", "ˈeːk lˈaːkʰ teːˈiːs hazˈaːr cˈaːrsˈaʊ cʰˈappan")]);
}
