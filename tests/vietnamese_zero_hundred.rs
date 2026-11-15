// Vietnamese says the empty hundreds place of a group that follows a higher
// group ("two thousand zero hundred twenty-four", C `NUM_ZERO_HUNDRED`) and
// puts "lẻ" before a lone digit after hundreds (`NUM_HUNDRED_AND_DIGIT`).
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(text: &str) -> Option<String> {
    let t = Translator::new_default("vi").ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn zero_hundred_after_a_higher_group() {
    let Some(_) = ipa("1") else { return };
    let cases = [
        ("2024", "hˈaːɪ1 ŋˈaː2n xˌo1ŋtʃˈa1m hˈaːɪ1 mˈyə7jbˈoɜn"),
        ("1007", "mˈo6t̪ ŋˈaː2n xˌo1ŋtʃˈa1mlˌi1ɲ bˈaɪ4"),
        ("1.007", "mˈo6t̪ ŋˈaː2n xˌo1ŋtʃˈa1mlˌi1ɲ bˈaɪ4"),
        ("1000", "mˈo6t̪ ŋˈaː2n xˌo1ŋtʃˈa7m"),
        ("0.007", "xˌo1ŋ ŋˈaː2n xˌo1ŋtʃˈa1mlˌi1ɲ bˈaɪ4"),
    ];
    for (text, want) in cases {
        assert_eq!(ipa(text).as_deref(), Some(want), "vi {text}");
    }
}

#[test]
fn li_before_a_lone_digit_after_hundreds() {
    let Some(_) = ipa("1") else { return };
    assert_eq!(ipa("107").as_deref(), Some("mˈo6t̪tʃˈa1mlˌi1ɲ bˈaɪ4"));
}
