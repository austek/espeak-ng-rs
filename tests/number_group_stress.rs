// A number of more than four digits is spoken as one word per group of three
// digits, and each word is stressed on its own: the conjunction that opens a
// later group takes the secondary stress with it. Expected strings are
// espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(text: &str) -> Option<String> {
    let t = Translator::new_default("ar").ok()?;
    t.text_to_ipa(text).ok()
}

fn check(cases: &[(&str, &str)]) {
    let Some(_) = ipa("1") else { return };
    for (text, want) in cases {
        assert_eq!(ipa(text).as_deref(), Some(*want), "ar {text}");
    }
}

#[test]
fn conjunction_opening_a_later_group_is_stressed() {
    check(&[
        (
            "222222",
            "mˌiʔatˈaːnwa ʔiθnˈaːnwaʕiʃrˈuːn ʔˈalf wˌamiʔatˈaːnwa ʔiθnˈaːnwaʕiʃrˈuːn",
        ),
        ("201201", "mˌiʔatˈaːnwa wˈaːħid ʔˈalf wˌamiʔatˈaːnwa wˈaːħid"),
        ("300300", "θalˈaːθumˌiʔa ʔˈalf wˌaθalˈaːθumˌiʔa"),
        (
            "12345",
            "ʔˈiθnˈaː ʕʕˌaʃar ʔˈalf wˌaθalˈaːθumˌiʔawa xˈamsawˌaʔarbˌaʕʕˈuːn",
        ),
    ]);
}

#[test]
fn millions_group_is_stressed_on_its_own() {
    check(&[
        (
            "2222222",
            "mˈiljuːnˌaːn wˌamiʔatˈaːnwa ʔiθnˈaːnwaʕiʃrˈuːn ʔˈalf wˌamiʔatˈaːnwa ʔiθnˈaːnwaʕiʃrˈuːn",
        ),
        (
            "1234567",
            "miljˌuːn wˌamiʔatˈaːnwa ʔˈarbaʕʕˌawaθˌalaːθˈuːn ʔˈalf waxˈamsumˌiʔawa sˈabʕʕˌawasitːˈuːn",
        ),
    ]);
}

#[test]
fn groups_without_a_leading_conjunction_are_unchanged() {
    check(&[
        ("123456", "mˈiʔawˌa θalˈaːθawˌaʕiʃrˈuːn ʔˈalf waʔˈarbaʕʕˌumiʔˌawa sˈitːawˌaxamsˈuːn"),
        ("777777", "sˈabʕʕˌumiʔˌawa sˈabʕʕˌawasˌabʕʕˈuːn ʔˈalf wasˈabʕʕˌumiʔˌawa sˈabʕʕˌawasˌabʕʕˈuːn"),
        ("1000", "ʔˈalf"),
    ]);
}
