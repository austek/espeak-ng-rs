// Languages whose `tr_languages.c` setup sets `spelling_stress` (Slovak,
// Latvian, Tamil, Kannada, Malayalam beside the ones already covered): a
// spelled-out word keeps only its first primary stress. Expected strings are
// from C espeak-ng master f0fb11a8.

use espeak_ng::Translator;

const CASES: &[(&str, &str, &str)] = &[
    ("sk", "SNS", "ˈesˌenˌes"),
    ("sk", "bcd", "bˈeːtsˌeːdˌeː"),
    ("sk", "ŠKD", "ˈeʃkˌaːdˌeː"),
    ("sk", "čšž", "tʃˈeːˌeʒʒˌet"),
    ("lv", "LTV", "ˈelltˌeːʋˌeː"),
    ("lv", "pfrt", "pˈeːˌeffˌerrtˌeː"),
    ("lv", "bcd", "bˈeːt͡sˌeːdˌeː"),
    ("lv", "ščž", "ˈeʃʃt͡ʃˌeːʒˌeː"),
    ("lv", "krk", "kˈaːˌerrkˌaː"),
];

#[test]
fn spelled_words_keep_only_the_first_primary_stress() {
    for &(lang, word, want) in CASES {
        let Some(t) = Translator::new(lang, None).ok() else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        assert_eq!(t.text_to_ipa(word).unwrap_or_default(), want, "{lang} {word}");
    }
}
