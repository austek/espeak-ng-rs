// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <lang>`.
// hr and bs share Serbian's `LOPT_REGRESSIVE_VOICING = 0x3`.

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
fn croatian_and_bosnian_voice_a_stop_before_a_voiceless_consonant() {
    for lang in ["hr", "bs", "sr"] {
        assert_eq!(cli(lang, "dogs"), "d'oks", "{lang}");
    }
    let text = "The quick brown fox jumps over 12 lazy dogs.";
    let want = "th'E kv'idzk br**'ovn f'oks j'umps 'ovER dv'anaIst l'azI d'oks";
    assert_eq!(cli("hr", text), want);
    assert_eq!(cli("bs", text), want);
}
