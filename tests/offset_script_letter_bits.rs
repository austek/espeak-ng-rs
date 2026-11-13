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

#[test]
fn arabic_waw_after_waw_is_not_a_voiceless_context() {
    assert_eq!(cli("ar", "-x", "ووب"), "wwb");
    assert_eq!(cli("ar", "--ipa", "ووب"), "wwb");
}

#[test]
fn kurdish_arabic_word_with_doubled_waw_matches_c() {
    assert_eq!(cli("ku", "-x", "ڕووبار"), "(ar)wwb'a:R(ku)");
    assert_eq!(cli("ku", "--ipa", "ڕووبار"), "(ar)wwbˈaːr(ku)");
}
