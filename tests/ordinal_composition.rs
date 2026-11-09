// tests/ordinal_composition.rs
//
// Ordinal numbers are composed as C's `LookupNum3`/`LookupNum2` do from the
// language's `numbers`/`numbers2` switches: hundreds and a conjunction before
// the ordinal remainder ("hundred and first"), the ordinal suffix as a word of
// its own after round hundreds and thousands, the ordinal form of the tens in
// Spanish and Portuguese ("vigésimo primero"), `_ord20` after round tens, and
// only the last group of a long number taking the ordinal.  The port used to
// compose tens and units its own way and differed from C in every language
// that has ordinals.  Expected output is from C espeak-ng master.

use espeak_ng::Translator;

#[test]
fn ordinals_match_c() {
    let cases = [
        ("en", "100th", "wˈɒnhˈʌndɹɪd θ"),
        ("en", "101st", "wˈɒnhˈʌndɹɪdən fˈɜːst"),
        ("en", "121st", "wˈɒnhˈʌndɹɪdən twˈɛnti fˈɜːst"),
        ("en", "1000th", "wˈɒn θˈaʊzənd θ"),
        ("en", "1001st", "wˈɒn θˈaʊzənd ən fˈɜːst"),
        ("en", "1234th", "wˈɒn θˈaʊzənd tˈuːhˈʌndɹɪdən θˈɜːti fˈɔːθ"),
        ("en", "10000th", "tˈɛn θˈaʊzənd θ"),
        ("en", "1000000th", "wˈɒn mˈɪliən θ"),
        ("es", "21º", "βixˌɛsimˌopɾimˈɛɾo"),
        ("es", "30º", "tɾixˈɛsimo"),
        ("es", "100º", "θɛntˈɛsim o"),
        ("es", "101º", "θɛntˈɛsimˌo pɾimˈɛɾo"),
        ("es", "1234º", "mˈil ðuθɛntˈɛsimˌo tɾixˌɛsimokwˈaɾto"),
        ("es", "10000º", "djˈɛθ mˈil ˈo"),
        ("es", "1000000º", "ˈunmiʝˈon ˈo"),
        ("es", "21ª", "βixˌɛsimˌapɾimˈɛɾa"),
        ("pt", "21º", "vˌiʒˈɛzimˌʊpɹimˈeɪɾʊ"),
        ("pt", "100º", "sˌeɪŋtˈɛzim ʊ"),
        ("pt", "101º", "sˌeɪŋtˈɛzimˌʊ pɹimˈeɪɾʊ"),
        ("pt", "1234º", "mˈil duseɪŋtˈɛzimˌʊ tɹiʒˈɛzimʊkwˈaɾətʊ"),
        ("it", "4º", "kwˈaːrto"),
        ("it", "21º", "vˈentʊnˈɛzimo"),
        ("it", "100º", "tʃentˈɛzim o"),
        ("it", "1000º", "milˈɛzim o"),
        ("de", "der 21. tag", "dɛɾ ˈaɪn ʊnttsvˈantsɪçstə tˈɑːk"),
        ("de", "der 100. tag", "dɛɾ ˈaɪnhˈʊndɜtstə tˈɑːk"),
        ("de", "der 1000. tag", "dɛɾ ˈaɪn tˈaʊzənt tˈɑːk"),
        ("de", "der 101. tag", "dɛɾ ˈaɪnhˈʊndɜt ˈeːɾstə tˈɑːk"),
        ("nl", "21e", " ˈeːnɛntʋˌɪntəxstə"),
        ("nl", "100e", "hˈɔndɛrtstə"),
        ("nl", "1000e", "dˈœyzɛnt"),
        ("af", "21de", "ˈiənɛntwˈəntəxdə"),
        ("af", "100de", "ˈiənhˈɔnərtstə də"),
        ("el", "1ος", "prˈotos"),
        ("el", "21ος", "ˌikostˌosprˈotos"),
        ("fa", "1000th", "jˌek hezˈɑr om"),
        ("fa", "2000th", "dˌo hezˈɑr om"),
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
