// tests/ssml_say_as_characters_tone_digits.rs
//
// A character the voice names itself is spelled with the name `LookupLetter` gives, so a tone
// digit in a th letter name stays behind the vowel's stress mark when another character of the
// word is retranslated by another voice.  Expected values are what C espeak-ng master
// (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 3] = [
    ("th", "<speak><say-as interpret-as=\"characters\">A1</say-as></speak>", "(en)ˈeɪ(th) nˈu5enɡ", "(en)'eI_!(th) n'u5eng_!"),
    ("th", "<speak><say-as interpret-as=\"characters\">A2b</say-as></speak>", "(en)ˈeɪ(th) sˈo5nɡ (en)bˈiː(th)", "(en)'eI_!(th) s'o5ng_! (en)b'i:_!(th)"),
    ("th", "<speak><say-as interpret-as=\"characters\" format=\"glyphs\">A1</say-as></speak>", "(en)kˌapətəlˈeɪ(th) nˈu5enɡ", "(en)k,ap@-t@L'eI_!(th) n'u5eng_!"),
];

#[test]
fn tone_digit_follows_the_stress_mark() {
    for (lang, text, ipa, espeak) in CASES {
        assert_eq!(run(lang, "--ipa", text), ipa, "{lang} --ipa {text}");
        assert_eq!(run(lang, "-x", text), espeak, "{lang} -x {text}");
    }
}
