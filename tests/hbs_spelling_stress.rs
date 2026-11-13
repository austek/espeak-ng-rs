// Croatian and Bosnian share Serbian's `spelling_stress`: a spelled-out word
// keeps only its first primary stress. Expected strings are from C espeak-ng
// master f0fb11a8.

use espeak_ng::Translator;

#[test]
fn spelled_words_keep_only_the_first_primary_stress() {
    for (lang, cda) in [("bs", "tsˈədˌəˌæ"), ("hr", "tsˈədˌəˌæ"), ("sr", "tsˈədˌəˌɐ")] {
        let Some(t) = Translator::new(lang, None).ok() else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let cases = [("abk", "ˈabˌəkˌə"), ("cda", cda)];
        for (word, want) in cases {
            let got = t.text_to_ipa(word).unwrap_or_default();
            assert_eq!(got, want, "{lang} {word}");
        }
    }
}
