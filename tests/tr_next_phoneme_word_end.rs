// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v tr`.

fn cli(text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", "tr", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn next_phoneme_word_end_sees_a_vowel_initial_next_word() {
    assert_eq!(cli("ben iyiyim"), "b&n ij'ijIm");
    assert_eq!(cli("sen ona"), "s&n on'a");
    assert_eq!(cli("ben ev"), "b&n 'Ev");
}
