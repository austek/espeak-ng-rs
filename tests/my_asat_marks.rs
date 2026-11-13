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
fn asat_and_dot_below_are_read_as_pauses() {
    cases(
        "my",
        "-x",
        &[
            ("ပိုင်", "p'oN _|"),
            ("ခွင့်", "khw'aN _|_|"),
            ("မူပိုင်ခွင့်", "m'u1poN _| khw'aN _|_|"),
            ("ပိုင် ခွင့်", "p'oN _| khw'aN _|_|"),
        ],
    );
}

#[test]
fn pauses_show_as_spaces_in_ipa() {
    cases("my", "--ipa", &[("မူပိုင်ခွင့်", "mˈu1poŋ  khwˈaŋ")]);
}
