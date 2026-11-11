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
fn comma_groups_thousands_unless_the_language_uses_a_decimal_comma() {
    cases(
        "ku",
        "--ipa",
        &[
            ("1,000", "hɛzˈar"),
            ("1,000,000", "mɪljˈon"),
            ("12,345", "danzdˈɛ hɛzˈar usˈesˈɛdu tʃˈɪlupˈendʒ"),
        ],
    );
}

#[test]
fn symbols_english_cannot_read_by_word_stay_silent_in_running_text() {
    cases(
        "ku",
        "--ipa",
        &[
            ("a ° b", "a bˈe"),
            ("a × b", "a bˈe"),
            ("a © b", "a bˈe"),
            ("36.6°", "sˈiuʃˈɛʃ xal ʃˈɛʃ"),
            ("50 °C", "pendʒˈi dʒˈe"),
            ("a ± b", "a (en)plˈʌs ɔː mˈaɪnəs(ku) bˈe"),
            ("a ⅓ b", "a (en)wˈɒn θˈɜːd(ku) bˈe"),
        ],
    );
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
