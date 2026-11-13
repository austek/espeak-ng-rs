// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v vi` and `--ipa`.

fn cli(flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", "vi", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn vietnamese_vowel_letters_keep_a_word_from_being_spelled() {
    for (text, want) in [
        ("bị", "_b,i6_|"),
        ("chẳng", "tS,a4N_|"),
        ("đến", "d_,e3n_|"),
        ("để", "d_,e4_|"),
    ] {
        assert_eq!(cli("-x", text), want, "{text}");
    }
    assert_eq!(cli("--ipa", "bị"), "bˌi6");
}

#[test]
fn only_a_primary_stressed_toneless_syllable_takes_the_falling_tone() {
    assert_eq!(cli("-x", "cho"), "tS,O1_|");
    assert_eq!(cli("-x", "xin chào"), "s'i1n_| tS'a:2w_|");
    assert_eq!(cli("--ipa", "cho"), "tʃˌɔ1");
}
