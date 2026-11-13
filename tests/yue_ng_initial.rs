// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v <lang>` and `-q -x -v <lang>`.

fn cli(lang: &str, flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

fn cases(lang: &str, flag: &str, rows: &[(&str, &str)]) {
    for (text, want) in rows {
        assert_eq!(&cli(lang, flag, text), want, "{lang} {flag} {text:?}");
    }
}

#[test]
fn ng_before_a_vowel_is_a_plain_consonant_without_a_tone() {
    cases("yue", "-x", &[("我", "N'o5_|"), ("安", "N'o1n_|"), ("我哋", "N'o5_| d'ei2_|")]);
    cases("yue", "--ipa", &[("我", "ŋˈo5"), ("安", "ŋˈo1n"), ("我哋", "ŋˈo5 dˈei2")]);
}

#[test]
fn a_syllabic_ng_keeps_its_tone() {
    cases("yue", "-x", &[("五", "'ng5_|"), ("嗯", "'ng2_|")]);
    cases("yue", "--ipa", &[("五", "ˈnɡ5")]);
}
