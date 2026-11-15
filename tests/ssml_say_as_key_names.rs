// tests/ssml_say_as_key_names.rs
//
// `tts:key` (C `SAYAS_KEY`): a one-character word is spelled; a longer one is looked up and
// read under its `_`-prefixed name only, so a key name the dictionary lacks is read by the
// rules from `_<name>` (`ctrl` is `ktrl`).  Expected values are what C espeak-ng master
// (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

const CASES: [(&str, &str, &str, &str); 6] = [
    ("en", "ctrl", "ktrl", "ktrl"),
    ("en", "F1", "ˈɛf wˈɒn", "'Ef_! w'0n_!"),
    ("en", "tab", "tˈab", "t'ab_!"),
    ("en", "space", "spˈeɪs", "sp'eIs_!"),
    ("de", "tab", "tˌabuːlˈɑːtoːɜ", "t,abu:l'A:to:3_!"),
    ("en", "A", "ˈeɪ", "'eI_!"),
];

#[test]
fn key_names_are_looked_up_under_their_underscore_name() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .flat_map(|&(l, k, i, x)| [(l, "--ipa", i, k), (l, "-x", x, k)])
        .filter_map(|(l, f, want, k)| {
            let text = format!("<speak><say-as interpret-as=\"tts:key\">{k}</say-as></speak>");
            let got = run(l, f, &text);
            (got != want).then(|| format!("{l} {f} {k}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
