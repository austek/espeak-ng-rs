// The pause of a quote is C's `pre_pause_add`, which the next word keeps as the largest of
// the pauses before it, so two quotes between the same words pause once.  Values from
// upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v en`.

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
fn quotes_before_one_word_pause_once() {
    for (text, want) in [
        ("cat' 'dog", "k'at_:_: d'0g"),
        ("'ab' 'cd' ef' gh", "'ab_:_: s,i:d'i:_:_: 'Ef_:_: dZ,i:;'eItS"),
        ("cat\" \"dog", "k'at_:_:_:_: d'0g"),
        ("cat) (dog", "k'at_:_:_:_: d'0g"),
    ] {
        assert_eq!(cli(text), want, "{text:?}");
    }
}
