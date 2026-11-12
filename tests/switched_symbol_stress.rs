// A symbol read by another voice (`&` → English "and") takes no tonic stress of
// its own while the clause goes on after it.  Values from upstream espeak-ng
// master (f0fb11a8), `espeak-ng -q -x -v <lang>`.

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
fn a_switched_symbol_before_more_words_is_unstressed() {
    for (lang, text, want) in [
        ("ku", "a & b", "a (en)_and(ku) b'e"),
        ("ku", "a @ b", "a (en)_at_(ku) b'e"),
        ("ku", "& b", "(en)_and(ku) b'e"),
        ("ku", "b & b", "b'e (en)_and(ku) b'e"),
        ("ku", "&", "(en)'amp@s,and(ku)"),
    ] {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}
