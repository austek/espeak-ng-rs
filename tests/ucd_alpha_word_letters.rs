// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v en` with ESPEAK_DATA_PATH set.

fn cli(text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", "en", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn a_letter_missing_from_the_c_unicode_tables_does_not_continue_a_word() {
    assert_eq!(cli("ab\u{0e86}cd"), "'ab s,i:d'i:");
}

#[test]
fn a_mark_missing_from_the_c_unicode_tables_does_not_continue_a_word() {
    assert_eq!(cli("ab\u{0897}cd"), "'ab s,i:d'i:");
}
