// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v hak`.

fn cli(text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", "hak", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn a_rule_switch_to_english_reads_the_whole_latin_word_in_english() {
    let rows = [
        ("abc", "(en),eI1b,i:1s'i:1(hak)"),
        ("ab", "(en)'a1b(hak)"),
        ("ac", "(en),eI1s'i:1(hak)"),
        ("abd", "(en)'a1bd(hak)"),
        ("abcd", "(en)'a1bkd(hak)"),
    ];
    for (text, want) in rows {
        assert_eq!(cli(text), want, "{text:?}");
    }
}

#[test]
fn a_word_that_starts_with_a_switching_letter_is_spelled() {
    assert_eq!(cli("bcd"), "_:(en)b,i:1_:s,i:1_:d'i:1(hak)");
    assert_eq!(cli("cde"), "_:(en)s'i:1 _:d'i:1(hak) 'e4");
}
