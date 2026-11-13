// A lone digit of another script, as a word of its own, is spelled with its
// script name rather than dropped.  Values from upstream espeak-ng master
// (f0fb11a8), `espeak-ng -q --ipa -v <lang>`.

fn cli(lang: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "--ipa", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn a_lone_foreign_digit_is_spelled() {
    let cases = [
        ("en", "abc ٥ def", "ˌeɪbˌiːsˈiː ˈaɹəbɪk(ar)xˈamsa(en) dˈɛf"),
        ("ku", "a ٣ b", "a (en)ˈaɹəbɪk(ku)θalˈaːθa bˈe"),
        ("en", "x٢y", "ˈɛks ˈaɹəbɪk(ar)ʔiθnˈaːn(en) wˈaɪ"),
    ];
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|&(lang, text, want)| {
            let got = cli(lang, text);
            (got != want).then(|| format!("{lang} {text:?}\n  got  {got}\n  want {want}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
