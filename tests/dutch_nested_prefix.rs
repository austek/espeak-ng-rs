// C peels every standard prefix off a word, then stresses the whole string once
// (or the innermost stem alone). The vowel-initial break therefore depends on
// the whole word, never on an inner prefix. Expected mnemonics are from C
// espeak-ng master.

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
fn stem_of_a_nested_prefix_gets_no_break_of_its_own() {
    check(&[
        ("herontdekken", "hErOntd'Ek@n"),
        ("herontdekt", "hErOntd'Ekt"),
        ("verontschuldigen", "v@rOntsx'8ld@Q@n"),
        ("verontschuldiging", "v@rOntsx'8ldiQ,IN"),
    ]);
}

#[test]
fn vowel_initial_word_with_stressed_inner_prefix_breaks_at_the_word_start() {
    check(&[("onopgelost", "_|On'OpQ@l,Ost"), ("onuitstaanbaar", "_|On'Wytst,a:nba:r")]);
}
