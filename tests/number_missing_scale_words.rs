// A scale word the language does not define (`_0M4` past "billion", `_0M3` in
// Spanish) is replaced the way `LookupThousands` does: the next scale down, or
// "thousand" again, and the all-zero groups after it name their scale. A unit
// before "thousand" takes its `_<n>a` form even after hundreds.
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(lang: &str, text: &str) -> Option<String> {
    let t = Translator::new_default(lang).ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn missing_scale_words_fall_back_to_a_lower_scale() {
    if ipa("en", "1").is_none() {
        return;
    }
    let cases = [
        ("ar", "1000000000000", "ʔˈalf mˈiljaːr"),
        (
            "ar",
            "12345678901234",
            "ʔˈiθnˈaː ʕʕˌaʃar ʔˈalf wˌaθalˈaːθumˌiʔawa xˈamsawˌaʔarbˌaʕʕˈuːn miljˌaːr wasˈitːumˌiʔawˌa θamˈaːnijˌawasˌabʕʕˈuːn miljuːn watˈisʕʕˌumiʔˌawa wˈaːħid ʔˈalf wˌamiʔatˈaːnwa ʔˈarbaʕʕˌawaθˌalaːθˈuːn",
        ),
        ("fa", "1000000000000", "jˌek hezˈɑro miljˈɑrdo"),
        ("fr", "1000000000000", "mˈil miljˈaʁ"),
        ("fr", "2000000000000", "dø mˈil miljˈaʁ"),
        (
            "fr",
            "1234567890123",
            "mˈil døsɑ̃ tʁɑ̃tkatʁ miljˈaʁ sɛ̃ksɑ̃ swasɑ̃tsɛt miljˈɔ̃ yisɑ̃ katʁəvɛ̃di mˈil sɑ̃ vɛ̃ttʁwˈa",
        ),
        ("es", "1000000000", "ˈuno mˈil miʝˈones"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}

#[test]
fn unit_after_hundreds_takes_the_combining_form_before_thousand() {
    if ipa("en", "1").is_none() {
        return;
    }
    let cases = [
        ("de", "101000", "ˈaɪnhˈʊndɜt ˈaɪn tˈaʊzənt"),
        ("de", "901000", "nˈɔønhˈʊndɜt ˈaɪn tˈaʊzənt"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(ipa(lang, text).as_deref(), Some(want), "{lang} {text}");
    }
}
