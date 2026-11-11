// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v <lang>` and `-q -x -v <lang>`.

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
fn bracket_after_a_clause_break_keeps_the_break() {
    cases("ku", "--ipa", &[("Wî ji min re got: \"were\".", "wi ʒɪ mˈɪn rɛ ɡot|wɛɾˈɛ")]);
    cases("ku", "-x", &[("a: \"b\"", "'a|_:_:b'e"), ("a: (\"b\"", "'a|_:_:b'e")]);
}

#[test]
fn adjacent_brackets_pause_once() {
    cases(
        "ku",
        "-x",
        &[
            ("a (\"b\"", "a_:_: b'e"),
            ("a ((\"b\"", "a_:_: b'e"),
            ("a ( b ) ( c", "a_:_: b'e_:_:_:_: dZ'e"),
        ],
    );
}
