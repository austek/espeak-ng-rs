// A period left after the last clause reports one empty clause (an empty line).
// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v <lang>`
// and `-q -x -v <lang>`.

fn cli(lang: &str, flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).replace('\n', "|")
}

/// (voice, text, `--ipa` stdout, `-x` stdout)
const CASES: &[(&str, &str, &str, &str)] = &[
    ("en", "a. .", "ˈeɪ||", "'eI||"),
    ("en", "a ; .", "ˈeɪ||", "'eI||"),
    ("en", "a: .", "ˈeɪ||", "'eI||"),
    ("en", "a . , .", "ˈeɪ||", "'eI||"),
    ("en", "1. .", "wˈɒn||", "w'0n||"),
    ("de", "a. .", "ˈɑː||", "_!'A:||"),
    ("ru", "a. .", "(en)ˈeɪ(ru)||", "(en)'eI(ru)||"),
    ("ar", "a. .", "(en)ˈeɪ(ar)||", "(en)'eI(ar)||"),
];

#[test]
fn period_after_the_last_clause_adds_one_empty_clause() {
    for (lang, text, ipa, x) in CASES {
        assert_eq!(&cli(lang, "--ipa", text), ipa, "{lang} {text:?}");
        assert_eq!(&cli(lang, "-x", text), x, "{lang} {text:?}");
    }
}
