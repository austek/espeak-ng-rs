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
fn a_stem_found_in_the_list_keeps_the_vowel_unreduced() {
    assert_eq!(cli("ali"), "al'i");
    assert_eq!(cli("eli"), "El'i");
    assert_eq!(cli("iyi"), "ij'i");
    assert_eq!(cli("iyi akşamlar"), "ij'i akSamL'aR");
    assert_eq!(cli("kali"), "kal'I");
}
