// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <lang>`.

fn cli(lang: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn three_or_more_dots_are_one_ellipsis_clause_break() {
    for (lang, text, want) in [
        ("ku", "a... b", "'a|b'e"),
        ("ku", "a... B", "'a|b'e"),
        ("ku", "a.... b", "'a|b'e"),
        ("ku", "a...b", "'a|b'e"),
        ("ku", "a ... b", "'a|b'e"),
        ("en", "a... b", "'eI|b'i:"),
        ("en", "a...b", "'eI|b'i:"),
    ] {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}

#[test]
fn two_dots_stay_two_dots() {
    assert_eq!(cli("ku", "a.. b"), "a b'e");
}
