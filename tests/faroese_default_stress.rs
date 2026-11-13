// Faroese leaves the stress rule at the translator default (penultimate); its
// tr_languages.c arm only sets number options. Expected strings are from C
// espeak-ng master f0fb11a8.

use espeak_ng::Translator;

#[test]
fn faroese_uses_penultimate_stress() {
    let Some(t) = Translator::new("fo", None).ok() else {
        eprintln!("[SKIP] no fo data");
        return;
    };
    let cases = [
        ("sveimaðari", "sʋaɟːmajˈaɹɪ"),
        ("rasistiski", "ɹˌasɪsdˈɪʃdʒɪ"),
        ("rosinunum", "ɹˌosiːnˈʊnʊn"),
    ];
    for (word, want) in cases {
        let got = t.text_to_ipa(word).unwrap_or_default();
        assert_eq!(got, want, "{word}");
    }
}
