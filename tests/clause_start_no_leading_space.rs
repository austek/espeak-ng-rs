// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v fr` and `-q -x -v fr`.
// C writes no space before the first word of a clause, even after opening pauses.

fn cli(flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", "fr", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).replace('\n', "|")
}

#[test]
fn opening_quote_then_space_adds_no_space() {
    assert_eq!(cli("-x", "« Merci"), "_:_:mErs'i|");
    assert_eq!(cli("--ipa", "« Merci"), "mɛʁsˈi|");
}

#[test]
fn closing_quote_then_word_in_another_voice_adds_no_space() {
    assert_eq!(cli("-x", "Hello!\" She said"), "El'o|_:_:(en)Si:(fr) s'Ed|");
    assert_eq!(cli("--ipa", "Hello!\" She said"), "ɛlˈo|(en)ʃiː(fr) sˈɛd|");
}
