// Tens joined to units keep only one primary stress when the language sets
// `NUM_SINGLE_STRESS`: the earlier ones become secondary.
//
// Values from C espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn check(lang: &str, cases: &[(&str, &str)]) {
    let Ok(t) = Translator::new(lang, Some(std::path::Path::new("espeak-ng-data"))) else {
        eprintln!("[SKIP] no {lang} data");
        return;
    };
    for (text, ipa) in cases {
        assert_eq!(t.text_to_ipa(text).ok().as_deref(), Some(*ipa), "{lang} {text}");
    }
}

#[test]
fn spanish_tens_and_units() {
    check(
        "es",
        &[
            ("34", "tɾˌeɪntaikwˈatɾo"),
            ("41", "kwaɾˌɛntaiˈuno"),
            ("96", "noβˌɛntaisˈeɪs"),
            ("1234", "mˈil ðosθjˈentos tɾˌeɪntaikwˈatɾo"),
        ],
    );
}
