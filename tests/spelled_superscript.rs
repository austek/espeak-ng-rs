// tests/spelled_superscript.rs
//
// An unpronounceable word is spelled letter by letter, and a super- or
// subscript letter in it (`Mª`, `2ºb`) is spelled as its base letter.  A word
// the dictionary lists (`Mª` in Spanish) is not spelled.  Expected output is
// from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn superscript_letters_are_spelled_inside_a_word() {
    let cases = [
        ("fr", "Mª", "ˌɛmˈa"),
        ("fr", "Dª", "dˌeˈa"),
        ("fr", "Mª José", "ˌɛmˈa ʒozˈe"),
        ("fr", "2ºb", "dˈø dəɡʁˌebˈe"),
        ("nl", "Mª", "ˌɛmˈaː"),
        ("nl", "2ºb", "tʋˈeː ˌoːbˈeː"),
        ("de", "Mª José", "ˌɛmˈɑː joːzˈeː"),
        ("de", "2ºb", "tsvˈaɪ ˌoːbˈeː"),
        ("it", "Mª", "ˌɛmmeˈaː"),
        ("it", "Dª", "d̪ˌiːˈaː"),
        ("es", "Dª", "dˌeˈa"),
        ("es", "2ºb", "dˈos ˌoβˈe"),
        ("es", "Mª", "maɾˈia"),
        ("es", "Mª José", "maɾˈia xosˈe"),
    ];
    let mut failures = Vec::new();
    for (lang, text, expected) in cases {
        let Ok(t) = Translator::new(lang, None) else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        let got = t.text_to_ipa(text).unwrap();
        if got != expected {
            failures.push(format!("{lang} {text:?}: got {got:?}, C says {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
