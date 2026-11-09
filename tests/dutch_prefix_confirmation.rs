// C confirms a standard prefix by finding the word's suffix, removing it, and
// checking that the prefix rule still matches the remainder. If it no longer
// does, the word keeps the suffix and drops the prefix. Expected mnemonics are
// from C espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("nl_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", "nl", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', " "))
}

fn check(cases: &[(&str, &str)]) {
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no nl data");
            return;
        };
        assert_eq!(got, *want, "{text}");
    }
}

#[test]
fn prefix_dropped_when_the_suffix_leaves_it_unrecognised() {
    check(&[
        ("openlijk", "_!'o:p@nl@k"),
        ("ontbijten", "Ontb'EIt@n"),
        ("samenwerken", "s'a:m@nv#,Erk@n"),
    ]);
}

#[test]
fn prefix_kept_when_it_survives_suffix_removal() {
    check(&[("geopend", "Q@_!'o:p@nt"), ("beloven", "b@l'o:v@n")]);
}
