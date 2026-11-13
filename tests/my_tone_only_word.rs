// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v my` and `-q -x -v my`.

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
fn a_lone_tone_sign_is_not_spelled_as_a_letter() {
    cases(
        "my",
        "-x",
        &[
            ("လေပူဖောင်း", "l,e3pu1ph'au2N _|"),
            ("လိင်ပြောင်းခြင်း သင်္ကေတ", "l'iN _| pr'au2N _| khrN _| DN _| k'e3t"),
            ("စမ်းသပ်ခန်း ဝတ်စုံ", "sm _| Dp _| khn _| wt _| s'u3a"),
        ],
    );
    cases("my", "--ipa", &[("လေပူဖောင်း", "lˌeɜpu1phˈau2ŋ")]);
}
