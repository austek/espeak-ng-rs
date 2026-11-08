// In "einundzwanzig" the unit one is the list entry `_1a`, a stressed word of
// its own, not the dictionary word "ein". Expected strings come from C
// espeak-ng master (f0fb11a).

use std::path::Path;
use std::process::Command;

fn ipa(text: &str) -> String {
    let bin = env!("CARGO_BIN_EXE_espeak_cli");
    let out = Command::new(bin)
        .args(["-q", "--ipa", "-v", "de", "--", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn check(cases: &[(&str, &str)]) {
    if !Path::new("espeak-ng-data/de_dict").exists() {
        eprintln!("[SKIP] no de data");
        return;
    }
    for (text, want) in cases {
        assert_eq!(&ipa(text), want, "de {text:?}");
    }
}

#[test]
fn unit_one_before_tens_is_stressed() {
    check(&[
        ("21", "ˈaɪn ʊnttsvˈantsɪç"),
        ("31", "ˈaɪn ʊntdɾˈaɪsɪç"),
        ("91", "ˈaɪn ʊntnˈɔøntsɪç"),
        ("121", "ˈaɪnhˈʊndɜt ˈaɪn ʊnttsvˈantsɪç"),
        ("1021", "ˈaɪn tˈaʊzənt ˈaɪn ʊnttsvˈantsɪç"),
        ("um 21 uhr", "ʊm ˈaɪn ʊnttsvˈantsɪç ˈuːɾ"),
        ("71", "ˈaɪn ʊntzˈiːptsɪç"),
    ]);
}

#[test]
fn other_units_and_round_numbers_are_unchanged() {
    check(&[
        ("22", "tsvˈaɪ ʊnttsvˈantsɪç"),
        ("20", "tsvˈantsɪç"),
        ("100", "ˈaɪnhˈʊndɜt"),
        ("101", "ˈaɪnhˈʊndɜt ˈaɪns"),
        ("1000", "ˈaɪn tˈaʊzənt"),
    ]);
}
