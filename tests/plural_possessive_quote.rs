// An apostrophe right after an `s` outside a quoted stretch adds no pause. Values from upstream
// espeak-ng master (f0fb11a8), `espeak-ng -q -x -v en`.

fn cli(text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", "en", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn a_plural_possessive_quote_adds_no_pause() {
    for (text, want) in [
        ("dogs' bones", "d'0gz b'oUnz"),
        ("Thomas' car", "t'0m@s k'A@"),
        ("boys' toys", "b'OIz t'OIz"),
        ("ladies' room", "l'eIdiz r'u:m"),
        ("dogs' and cats'", "d'0gz and k'ats"),
    ] {
        assert_eq!(cli(text), want, "{text:?}");
    }
}

#[test]
fn a_quote_that_closes_a_quoted_stretch_still_pauses() {
    for (text, want) in [
        ("'dogs' bones", "d'0gz_:_: b'oUnz"),
        ("he said 'dogs' now", "hi: s'Ed_:_: d'0gz_:_: n'aU"),
    ] {
        assert_eq!(cli(text), want, "{text:?}");
    }
}
