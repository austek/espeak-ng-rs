// Arabic puts "wa" (and) in front of a scaled group that has no hundreds when a
// higher group came before it (C `NUM_THOUSAND_AND`): "one million AND
// twenty-one thousand". A count with a word of its own ("five thousand") and a
// group that opens a number take no conjunction. Expected strings are
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
fn conjunction_before_a_bare_scaled_group() {
    check(&[
        ("1021000", "miljˌuːn wa wˈaːħidwˌaʕiʃrˈuːn ʔˈalf"),
        ("3045000", "θˌalaːθat mˈalaːjˌiːn wa xˈamsawˌaʔarbˌaʕʕˈuːn ʔˈalf"),
        ("5071000", "χˌamsat mˈalaːjˌiːn wa wˈaːħidwˌasabʕʕˈuːn ʔˈalf"),
        (
            "12034056",
            "ʔˈiθnˈaː ʕʕˌaʃar miljuːn wa ʔˈarbaʕʕˌawaθˌalaːθˈuːn ʔˈalf wa sˈitːawˌaxamsˈuːn",
        ),
        ("2000021000", "mˈiljaːɹˌaːn  wa wˈaːħidwˌaʕiʃrˈuːn ʔˈalf"),
    ]);
}

#[test]
fn conjunction_before_a_bare_group_after_hundreds_of_millions() {
    check(&[(
        "633062349",
        "sˈitːumˌiʔawˌa θalˈaːθawˌaθalaːθˈuːn miljuːn wˌa ʔiθnˈaːnwasitːˈuːn ʔˈalf wˌaθalˈaːθumˌiʔawa tˈisʕʕˌawaʔˌarbaʕʕˈuːn",
    )]);
}

#[test]
fn no_conjunction_when_the_group_opens_the_number_or_has_its_own_word() {
    check(&[
        ("21000", "wˈaːħidwˌaʕiʃrˈuːn ʔˈalf"),
        ("71000", "wˈaːħidwˌasabʕʕˈuːn ʔˈalf"),
        ("1005000", "miljˌuːn xˈamsatˌu ʔaːlˈaːf"),
        ("21045", "wˈaːħidwˌaʕiʃrˈuːn ʔˈalf wa xˈamsawˌaʔarbˌaʕʕˈuːn"),
    ]);
}
