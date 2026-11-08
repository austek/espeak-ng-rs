// A group of zero digits inside a long number is still a word of its own: it
// carries only the pause that closes every inexact group, so the IPA keeps the
// extra space. Expected strings are espeak-ng master (f0fb11a8) output.

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
fn zero_group_between_two_spoken_groups() {
    check(&[
        ("1000001", "miljˌuːn  wa wˈaːħid"),
        ("1000200", "miljˌuːn  wˌamiʔatˈaːn"),
        ("1000300", "miljˌuːn  wˌaθalˈaːθumˌiʔa"),
        ("2000300", "mˈiljuːnˌaːn  wˌaθalˈaːθumˌiʔa"),
    ]);
}

#[test]
fn two_zero_groups_leave_two_extra_spaces() {
    check(&[("3000000001", "θˌalaːθat mˈiljaːɹˌaːt   wa wˈaːħid")]);
}

#[test]
fn numbers_with_every_group_spoken_keep_single_spaces() {
    check(&[
        ("100001", "mˈiʔa ʔˈalf wa wˈaːħid"),
        ("200001", "mˌiʔatˈaːn ʔˈalf wa wˈaːħid"),
        ("1000000", "miljˈuːn"),
    ]);
}
