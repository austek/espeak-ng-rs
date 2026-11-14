// A quote after a word's `s` adds no pause unless it closes a quoted stretch (C's
// `single_quoted` in `ReadClause`: an in-word apostrophe or a new clause resets it, the
// check on the `s` ignores case).  Values from upstream espeak-ng master (f0fb11a8),
// `espeak-ng -q -x -v en`.

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
fn plural_quote_follows_the_single_quoted_state() {
    for (text, want) in [
        ("'Bob's cats' toys", "b'0bz k'ats t'OIz"),
        ("'don't dogs' bones", "d'oUnt d'0gz b'oUnz"),
        ("he said 'it's the boss' car'", "hi: s'Ed_:_: Its D@ b'0s k'A@"),
        ("DOGS' BONES", "d'0gz b'oUnz"),
        ("dogz' bones", "d'0gz_:_: b'oUnz"),
        ("'yes' cats' toys", "j'Es_:_: k'ats t'OIz"),
        ("a 'b' dogs' x", "a#_:_: b'i:_:_: d'0gz 'Eks"),
        ("'cats', dogs' bones", "k'ats|d'0gz b'oUnz"),
        ("'dogs' cats' x", "d'0gz_:_: k'ats 'Eks"),
        ("'' dogs' x", "d'0gz_:_: 'Eks"),
        ("'5 dogs' x", "f'aIv d'0gz_:_: 'Eks"),
        ("('dogs' x)", "_:_:d'0gz 'Eks"),
        ("'12' dogs' x", "tw'Elv_:_: d'0gz_:_: 'Eks"),
    ] {
        assert_eq!(cli("en", text), want, "{text:?}");
    }
}

#[test]
fn plural_quote_keeps_a_switched_run_open() {
    for (lang, text, want) in [
        ("ru", "It's the dogs' bowl.", "(en)Its D@ d'0gz b'oUl(ru)"),
        ("ar", "It's the dogs' bowl.", "(en)Its D@ d'0gz b'oUl(ar)"),
    ] {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}
