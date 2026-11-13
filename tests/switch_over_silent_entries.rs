// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v sjn` and `-q -x -v sjn`.
// A word the voice does not speak (sjn has no number rules) does not close the
// language switch of the symbols around it.

fn cli(flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", "sjn", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn silent_numbers_between_switched_symbols_share_one_group() {
    assert_eq!(cli("-x", "+ 4 +"), "(en)pl'Vs pl'Vs(sjn)");
    assert_eq!(cli("-x", "/ 4 /"), "(en)sl'aS sl'aS(sjn)");
    assert_eq!(cli("-x", "+ 4 ="), "(en)pl'Vs 'i:kw@Lz(sjn)");
    assert_eq!(cli("-x", "+ 4 = +"), "(en)pl'Vs 'i:kw@Lz pl'Vs(sjn)");
    assert_eq!(cli("--ipa", "25/12/2024"), "(en)slˈaʃ slˈaʃ(sjn)");
    assert_eq!(cli("-x", "25/12/2024"), "(en)sl'aS sl'aS(sjn)");
}
