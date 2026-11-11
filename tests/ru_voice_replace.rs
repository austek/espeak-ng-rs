// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v ru`.

fn cli(flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", "ru", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn replace_matches_the_phoneme_the_rules_wrote() {
    assert_eq!(cli("-x", "двадцатой"), "dva_:ts'AtVj");
    assert_eq!(cli("-x", "двадцатая"), "dva_:ts'AtVja");
    assert_eq!(cli("-x", "у двадцатой"), "u dva_:ts'AtVj");
}

#[test]
fn unstressed_conjunction_a_takes_the_replacement() {
    assert_eq!(cli("-x", "а она"), "a# Vn'A");
    assert_eq!(cli("-x", "кто а он"), "kt'o a# 'on_");
    assert_eq!(cli("--ipa", "а она"), "a ʌnˈɑ");
}
