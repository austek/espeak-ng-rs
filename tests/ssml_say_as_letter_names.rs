// tests/ssml_say_as_letter_names.rs
//
// C `LookupLetter` reads a spelled letter through the rules with no dictionary flags, so a
// flags-only list entry (es `u` is `$u+`) does not unstress the letter's name.  Expected
// values are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", "es", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

const CASES: [(&str, &str, &str); 4] = [
    ("ONU", "ˌoˌeneˈu", ",o_|,ene_|'u_!"),
    ("UN", "ˌuˈɛne", ",u_|'Ene_!"),
    ("NU", "ˌeneˈu", ",ene_|'u_!"),
    ("UNU", "ˌuˌeneˈu", ",u_|,ene_|'u_!"),
];

#[test]
fn spanish_letter_u_keeps_its_stress_when_spelled() {
    if !std::path::Path::new("espeak-ng-data/es_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .flat_map(|&(t, i, x)| [("--ipa", i, t), ("-x", x, t)])
        .filter_map(|(f, want, t)| {
            let text = format!("<speak><say-as interpret-as=\"characters\">{t}</say-as></speak>");
            let got = run(f, &text);
            (got != want).then(|| format!("{f} {t}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
