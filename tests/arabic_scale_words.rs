// A group of N thousands/millions/billions has a complete word of its own when
// the dictionary defines `_NM1`, `_NM2`, `_NM3` (Arabic dual and plural forms);
// that word replaces the count and the generic scale word.
// Expected strings are espeak-ng master (f0fb11a8) output.

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
fn dual_and_plural_thousands() {
    check(&[
        ("2000", "ʔalfˈaːn"),
        ("2200", "ʔalfˈaːn wamˌiʔatˈaːn"),
        ("3000", "θalˈaːθatˌu ʔaːlˈaːf"),
        ("4000", "ʔarbˈaʕʕˌatu ʔaːlˈaːf"),
        ("5000", "xˈamsatˌu ʔaːlˈaːf"),
        ("8000", "θˌamaːnˈijatˌu ʔaːlˈaːf"),
        ("10000", "ʕʕˌaʃratˌu ʔaːlˈaːf"),
        ("3100", "θalˈaːθatˌu ʔaːlˈaːf wamˈiʔa"),
    ]);
}

#[test]
fn dual_and_plural_millions_and_billions() {
    check(&[
        ("2000000", "mˈiljuːnˌaːn"),
        ("3000000", "θˌalaːθat mˈalaːjˌiːn"),
        ("10000000", "ʕʕʃˌaɹat mˈalaːjˌiːn"),
        ("2000000000", "mˈiljaːɹˌaːn"),
        ("3000000000", "θˌalaːθat mˈiljaːɹˌaːt"),
    ]);
}

#[test]
fn counts_without_a_scale_word_of_their_own_are_unchanged() {
    check(&[
        ("1000", "ʔˈalf"),
        ("1001", "ʔˈalf wa wˈaːħid"),
        ("11000", "ʔˈaħadˌa ʕʕˌaʃar ʔˈalf"),
        ("21000", "wˈaːħidwˌaʕiʃrˈuːn ʔˈalf"),
        ("200000", "mˌiʔatˈaːn ʔˈalf"),
        ("1000000", "miljˈuːn"),
    ]);
}
