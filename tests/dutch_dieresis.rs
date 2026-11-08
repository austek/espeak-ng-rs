// A vowel with a dieresis (ë ï ü ö ä) that no rule covers is read as its plain
// vowel and translation continues from that letter (LOPT_DIERESES), instead of
// the letter being dropped.  Expected strings are from C espeak-ng master.

use espeak_ng::Translator;

fn check(lang: &str, cases: &[(&str, &str)]) {
    let Some(t) = Translator::new(lang, None).ok() else {
        eprintln!("[SKIP] no {lang} data");
        return;
    };
    for (word, want) in cases {
        let got = t.text_to_ipa(word).unwrap_or_default();
        assert_eq!(&got, want, "{word}");
    }
}

#[test]
fn dutch_dieresis_vowels_are_read() {
    check(
        "nl",
        &[
            ("ingrediënt", "ɪnɣrˌeːdiˈɛnt"),
            ("alexandrië", "ˌaːlɛksˈɑndriə"),
            ("reünie", "rˌeːynˈi"),
            ("coördinatie", "kˌoːɔrdinˈaːtsi"),
            ("ruïne", "ryˈinə"),
            ("financiën", "fˈinɑnʃən"),
            ("België", "bˈɛlɣiə"),
            ("Zoë", "zˈoːə"),
            ("geïnteresseerd", "ɣəˌɪntərɛsˈɪrt"),
            ("naïef", "naːˈif"),
            ("vacuüm", "vˈaːkyˌɵm"),
            ("egoïst", "ˌeːɣoːˈɪst"),
            ("mozaïek", "mˌoːzaːˈik"),
            ("tweeën", "tʋˈeːən"),
            ("oceaan", "ˌoːseːjˈaːn"),
        ],
    );
}

#[test]
fn afrikaans_dieresis_vowels_are_read() {
    check("af", &[("koördineer", "kuˌɔrdənˈiər"), ("Mosaïek", "mʊəsɐˈik")]);
}
