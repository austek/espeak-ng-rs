// Stray marks: a mark before any word is read by name like other leading
// punctuation, and a run of periods between words ends one clause without adding
// an empty one. Values from upstream espeak-ng master (f0fb11a8),
// `espeak-ng -q --ipa -v <lang>` and `-q -x -v <lang>`.

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
type Case = (&'static str, &'static str, &'static str, &'static str);

fn check(cases: &[Case]) {
    for (lang, text, ipa, x) in cases {
        assert_eq!(&cli(lang, "--ipa", text), ipa, "{lang} {text:?}");
        assert_eq!(&cli(lang, "-x", text), x, "{lang} {text:?}");
    }
}

#[test]
fn leading_inverted_question_mark_is_read_by_name_in_french() {
    let spoken = (
        "pwɛ̃ dɛ̃teʁoɡasjɔ̃ ɛ̃vɛʁsˈe ˈa|",
        "pwE~ dE~terogasjO~ E~vErs'e 'a|",
    );
    check(&[
        ("fr", "¿a", spoken.0, spoken.1),
        ("fr", "¿ a", spoken.0, spoken.1),
    ]);
}

#[test]
fn spaced_periods_between_words_end_one_clause() {
    check(&[
        ("en", "a . . b", "ˈeɪ|bˈiː|", "'eI|b'i:|"),
        ("en", "a. . b", "ˈeɪ|bˈiː|", "'eI|b'i:|"),
        ("en", "hello . . world", "həlˈəʊ|wˈɜːld|", "h@l'oU|w'3:ld|"),
        ("en", "1 . . 2", "wˈɒn|tˈuː|", "w'0n|t'u:|"),
        ("de", "a . . b", "ˈɑː|bˈeː|", "_!'A:|b'e:|"),
        ("fr", "a . . b", "ˈa|bˈe|", "'a|b'e|"),
    ]);
}

#[test]
fn marks_before_the_first_word_leave_no_empty_clause() {
    check(&[("en", ". , a", "ˈeɪ|", "'eI|")]);
}
