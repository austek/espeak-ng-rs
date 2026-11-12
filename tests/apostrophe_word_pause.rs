// A quote inside a word that the language's rules do not cover raises the pause
// before the next word (C's `pre_pause` in `TranslateRules`).  Values from
// upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <lang>`.

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
fn an_uncovered_quote_pauses_before_the_next_word() {
    for (lang, text, want) in [
        ("ku", "2010'an de hat", "d'U hEz'ar _u d'Eh 'an_:_: dE# h'at"),
        ("ku", "Roja 5'ê Adarê", "R'oZa p'endZ 'e_:_: ad'aRe"),
        ("ku", "a'b c d", "'ab_:_: dZ'e d'E"),
        ("tr", "1990'da geldi", "b'in d'okuzj'yz doks'an d'a_:_: J&ld'I"),
        ("de", "Ankara'da kaldı", "_!'ank,ARad,A:_:_: k'aldi:"),
        ("en", "don't go", "d'oUnt g'oU"),
    ] {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}
