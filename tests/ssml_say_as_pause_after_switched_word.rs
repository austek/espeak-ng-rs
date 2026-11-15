// tests/ssml_say_as_pause_after_switched_word.rs
//
// A spelled word that follows a word spoken in another voice starts with C's pre-pause, which
// C places after the switch back to the voice, because the switch is part of the word before it.
// Expected values are what C espeak-ng master (f0fb11a8) prints.

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
    (
        "hi",
        "<speak>word <say-as interpret-as=\"characters\">12</say-as> word</speak>",
        "(en)wˈɜːd(hi) ˌeːkdˈoː (en)wˈɜːd(hi)",
        "(en)w'3:d(hi)_:_: ,e:k_|d'o:_! (en)w'3:d(hi)",
    ),
    (
        "ru",
        "<speak>word <say-as interpret-as=\"characters\">12</say-as></speak>",
        "(en)wˈɜːd(ru) ʌdʲˌindvˈɑ",
        "(en)w'3:d(ru)_:_: V#d;,in__|dv'A_!",
    ),
    (
        "ar",
        "<speak>word <say-as interpret-as=\"characters\">12</say-as></speak>",
        "(en)wˈɜːd(ar) wˌaːħidʔiθnˈaːn",
        "(en)w'3:d(ar)_:_: w,a:Hid_|?iTn'a:n_!",
    ),
];

#[test]
fn pre_pause_of_a_spelled_word_follows_the_switch_back() {
    let mut failures = Vec::new();
    for (lang, text, ipa, x) in CASES {
        for (flag, want) in [("--ipa", ipa), ("-x", x)] {
            let got = run(lang, flag, text);
            if got != want {
                failures.push(format!("{lang} {flag} {text:?}\n  want {want:?}\n  got  {got:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
