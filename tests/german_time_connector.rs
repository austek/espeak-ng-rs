// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v de` and `-q -x -v de`.
// `de_rules` reads the colon of a time as an unstressed `u:r`.

fn cli(flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", "de", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn time_colon_is_an_unstressed_uhr() {
    assert_eq!(cli("-x", "14:30 Uhr"), "f'Irtse:n _|u:r dr'aIsIC _!'u:r");
    assert_eq!(cli("--ipa", "14:30 Uhr"), "fˈɪɾtseːn uːɾ dɾˈaɪsɪç ˈuːɾ");
    assert_eq!(cli("-x", "um 7:15"), "_!_|Um z'i:b@n _|u:r f'ynftse:n");
    assert_eq!(cli("--ipa", "um 7:15"), "ʊm zˈiːbən uːɾ fˈʏnftseːn");
}
