// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v ku` and `-q -x -v ku`.

fn cli(flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", "ku", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn unspaced_marks_do_not_break_the_clause() {
    for (text, ipa, x) in [
        ("a;b", "a bˈe", "a b'e"),
        ("Çi dikî,tu", "tʃˈɪ dɪcˈi tˈʊ", "tS'I dIc'i t'U"),
        ("1;2;3", "jˈɛk dˈʊ sˈe", "j'Ek d'U s'e"),
        ("a,,b", "a bˈe", "a b'e"),
        ("ez!tu", "ɛz tˈʊ", "Ez t'U"),
    ] {
        assert_eq!(cli("--ipa", text), ipa, "{text:?}");
        assert_eq!(cli("-x", text), x, "{text:?}");
    }
}

#[test]
fn a_comma_and_space_between_numbers_pauses() {
    assert_eq!(cli("-x", "1, 2"), "j'Ek|d'U");
    assert_eq!(cli("-x", "12, 34, 56"), "danzd'E#|s'i_utS'ar|pendZ'i_uS'ES");
    assert_eq!(cli("-x", "3,14"), "s'e tSaRd'E#");
}

#[test]
fn unspaced_question_mark_joins_the_letters() {
    assert_eq!(cli("--ipa", "a?b"), "ˈab");
    assert_eq!(cli("-x", "a?b"), "'ab");
    assert_eq!(cli("--ipa", "ez?tu"), "ɛztˈʊ");
}
