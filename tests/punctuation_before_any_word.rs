// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v en` and `-q -x -v en`.
// `ReadClause` ends a clause at punctuation only once the clause holds a letter
// or digit (`any_alnum`), so a run of marks between clauses adds no empty clause.

fn cli(flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", "en", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).replace('\n', "|")
}

#[test]
fn spaced_marks_after_a_clause_add_no_empty_clause() {
    for text in ["a, , b", "a , , b", "a, ; b", "a ? ? b", "a ; ; b", "a . . b"] {
        assert_eq!(cli("-x", text), "'eI|b'i:|", "{text:?}");
    }
    assert_eq!(cli("--ipa", "a, , b"), "ˈeɪ|bˈiː|");
    assert_eq!(cli("-x", "a ; , ; b"), "'eI|b'i:|");
}

#[test]
fn marks_before_the_first_word_add_no_empty_clause() {
    assert_eq!(cli("-x", ", , a"), "'eI|");
    assert_eq!(cli("--ipa", ", , a"), "ˈeɪ|");
    assert_eq!(cli("-x", ". . a"), "'eI|");
}

#[test]
fn a_second_exclamation_mark_is_spoken() {
    assert_eq!(cli("-x", "a ! ! b"), "'eI|_:'Ekskl@m,eIS@n_: b'i:|");
    assert_eq!(cli("--ipa", "a ! ! b"), "ˈeɪ|ˈɛkskləmˌeɪʃən bˈiː|");
}
