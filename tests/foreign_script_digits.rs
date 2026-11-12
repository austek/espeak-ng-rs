// C reads only ASCII digits as numbers.  A digit of another script is spoken
// only where the language's `.replace` table maps it to one (Arabic, Hindi);
// everywhere else it is dropped.  Expected strings are from upstream
// espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v <lang>`.

use espeak_ng::Translator;

#[test]
fn foreign_digits_are_silent_without_a_replace_table() {
    let cases = [
        ("en", "٢٠٢٤", ""),
        ("de", "٥٠٪", ""),
        ("ku", "٢٠٢٤", ""),
        ("ru", "۱۲۳", ""),
        ("es", "٣٤٥", ""),
        ("tr", "٢٠٢٤", ""),
        ("ar", "٣٤٥", "θalˈaːθumˌiʔawa xˈamsawˌaʔarbˌaʕʕˈuːn"),
        ("hi", "३४५", "tˈiːnsˈɔː paɪntˈaːlis"),
        ("ur", "٣٤٥", "tˈiːnsˈaʊ paɪŋtaːlˈiːs"),
    ];
    let mut failures = Vec::new();
    for (lang, text, expected) in cases {
        let Ok(t) = Translator::new(lang, None) else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let got = t.text_to_ipa(text).unwrap();
        if got.trim() != expected {
            failures.push(format!("{lang} {text:?}: got {got:?}, C says {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
