// `--sep` puts no separator between a clause's opening pauses and its first
// sound, and a closer after a mark is a clause of its own (one more empty
// line).  Values from upstream espeak-ng master (f0fb11a8).

fn cli(args: &[&str], text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(args)
        .args(["--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn separator_skips_the_gap_after_opening_pauses() {
    assert_eq!(
        cli(&["-q", "-x", "--sep=_", "-v", "en"], "\"Quote\" here"),
        "_:__:k_w_'oU_t__:__: h_'i@3\n"
    );
    assert_eq!(
        cli(&["-q", "-x", "--sep=|", "-v", "de"], "One, two, three"),
        "_!|'o:|n|@\nt|w|'o:\nt|r|'e:\n"
    );
}

#[test]
fn a_closer_after_the_last_mark_prints_an_empty_clause() {
    assert_eq!(cli(&["-q", "-x", "-v", "en"], "Come here!\""), "k'Vm h'i@3\n\n");
    assert_eq!(cli(&["-q", "--ipa", "-v", "en"], "Hello! \""), "həlˈəʊ\n\n");
    assert_eq!(cli(&["-q", "-x", "-v", "en"], "A 5\" pipe").lines().count(), 1);
}
