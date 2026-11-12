// Fuzz crash: a one-vowel word under a stress rule with S_FINAL_LONG indexed the
// vowel lengths at -1. Values from upstream espeak-ng master (f0fb11a8),
// `espeak-ng -q -x -v om`.

fn cli(lang: &str, text: &str) -> std::process::Output {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli")
}

#[test]
fn single_vowel_word_with_final_long_rule_does_not_panic() {
    for (text, want) in [("kk", "k,a:k'a:"), ("kk bb", "k,a:k'a: b,a:b'a:"), ("a", "'a:")] {
        let out = cli("om", text);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), want, "{text}");
    }
}
