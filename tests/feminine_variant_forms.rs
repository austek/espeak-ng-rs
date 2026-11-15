// A count before a thousand takes the `_<n>f` feminine form for every number
// that has one, not only 1 and 2 (C `LookupNum2` control bit 8): Greek
// "tesseris chiliades", Icelandic "fjögur thusund". Expected strings are
// espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn feminine_forms_beyond_two() {
    let Some(_) = ipa("el", "1") else { return };
    let cases = [
        ("el", "4073", "tˈeserˌis çiʎˈaðes ˌebdomˌindatrˈia"),
        ("el", "3000", "trˈis çiʎˈaðes"),
        ("is", "4073", "fjˈœːɣyrr# θˈuːsynd sjˈœːtiˌyɔːɣθr#ˈiːrr#"),
        ("is", "3000", "θr#jˈuː θˈuːsynd"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}
