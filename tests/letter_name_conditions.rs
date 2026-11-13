// A spelled letter is looked up with the voice's dictionary conditions, so
// Latin-American Spanish says `w` as "doble ve" (`?2 w`). Expected strings are
// from C espeak-ng master f0fb11a8.

use espeak_ng::Translator;

#[test]
fn spelled_letters_honour_dictionary_conditions() {
    let Some(t) = Translator::new("es-419", None).ok() else {
        eprintln!("[SKIP] no es-419 data");
        return;
    };
    let got = t.text_to_ipa("qw").unwrap_or_default();
    assert_eq!(got, "kˌuðˌoβleβˈe");
}
