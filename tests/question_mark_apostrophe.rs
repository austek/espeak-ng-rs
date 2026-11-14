// A `?` or `‘` between two letters may be a smart quote that lost its encoding, so C's
// `TranslateClause` reads it as an apostrophe inside the word.  Values from upstream
// espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <lang>`.

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
fn question_mark_between_letters_is_an_apostrophe() {
    for (lang, text, want) in [
        ("en", "a?b", "'a#b"),
        ("de", "a?b", "_!'Ap"),
        ("en", "x?y", "'Eks'aI"),
        ("en", "don\u{2018}t", "d'oUnt"),
        ("en", "what?Yes", "w'0taIz"),
        ("en", "it?s", "'Its"),
        ("en", "5?b", "f'aIv b'i:"),
    ] {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}
