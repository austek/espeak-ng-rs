// C ends a clause at punctuation only after a letter or digit was read in it, so
// punctuation with no word before it stays in the text and is spoken by name.
// A run of `?` and `!` is one character (the first). Expected strings are from
// C espeak-ng master f0fb11a8.

use espeak_ng::Translator;

#[test]
fn punctuation_before_any_word_is_spoken_by_name() {
    let cases = [
        ("en", "!", "ˈɛkskləmˌeɪʃən"),
        ("en", "!!", "ˈɛkskləmˌeɪʃən"),
        ("en", ":", "kˈəʊlən"),
        ("en", "! x", "ˈɛkskləmˌeɪʃən ˈɛks"),
        ("en", "(!)", "ˈɛkskləmˌeɪʃən"),
        ("en", "?!", ""),
        ("en", ",", ""),
        ("bs", "!", "ˈusklɪtʃnɪk"),
        ("bs", ":", "dvˈotætʃkæ"),
    ];
    for (lang, text, want) in cases {
        let Some(t) = Translator::new(lang, None).ok() else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let got = t.text_to_ipa(text).unwrap_or_default();
        assert_eq!(got.trim(), want, "{lang} {text:?}");
    }
}
