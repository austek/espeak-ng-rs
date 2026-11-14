// Armenian rules use the letter groups built from C's `hy_vowels`, `hy_consonants`
// and `hy_consonants2`, not the Latin default. Values from upstream espeak-ng
// master (f0fb11a8), `espeak-ng -q --ipa -v <lang>` and `-q -x -v <lang>`.

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
    ("hy", "շնորհակալություն", "ʃənˌorhakˌalutʰjˈun|", "S@n,orhak,alut#j'un|"),
    ("hy", "գրադարան", "ɡəɹˌadaɹˈan|", "g@r,adar'an|"),
    ("hyw", "բարեւ", "pʰaɹˈev|", "p#ar'ev|"),
    ("hyw", "Հայաստան", "hˌaɪasdˈan|", "h,aIasd'an|"),
    ("hyw", "աշխարհ", "aʃχˈarh|", "aSX'arh|"),
];

#[test]
fn armenian_rules_see_the_c_vowel_and_consonant_groups() {
    for (lang, text, ipa, x) in CASES {
        assert_eq!(&cli(lang, "--ipa", text), ipa, "{lang} {text:?}");
        assert_eq!(&cli(lang, "-x", text), x, "{lang} {text:?}");
    }
}
