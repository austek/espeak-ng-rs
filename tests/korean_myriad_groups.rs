// Korean groups digits in fours (`NUM2_MYRIADS`): the thousands digit of a
// group takes the scale-0 thousand word (cheon), not the myriad word (man).
// Expected strings are espeak-ng master (f0fb11a8) output.

use espeak_ng::Translator;

fn ipa(text: &str) -> Option<String> {
    let t = Translator::new_default("ko").ok()?;
    t.text_to_ipa(text).ok()
}

#[test]
fn thousands_digit_inside_a_myriad_group() {
    let Some(_) = ipa("1") else { return };
    let cases = [
        ("4073", "sˈɐ tʃhˈʌn tʃhˈirsˈipsˈɐm"),
        ("2963", "ˈi tʃhˈʌn ɡˈupˈɛq jˈuq sˈipsˈɐm"),
        ("1000", "tʃhˈʌn"),
        ("1234", "tʃhˈʌn ˈipˈɛq sˈɐm sˈipsˈɐ"),
        ("1,234", "tʃhˈʌn ˈipˈɛq sˈɐm sˈipsˈɐ"),
        ("7001", "tʃhˈir tʃhˈʌn ˈir"),
    ];
    for (text, want) in cases {
        assert_eq!(ipa(text).as_deref(), Some(want), "ko {text}");
    }
}

#[test]
fn myriads_above_the_first_group() {
    let Some(_) = ipa("1") else { return };
    assert_eq!(ipa("57081").as_deref().map(|s| s.contains("tʃhˈʌn")), Some(true));
}
