// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <lang>`.

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
fn an_english_letter_name_runs_its_phoneme_programs_inside_another_voice() {
    let rows = [
        ("ru", "(en)'Eks w'aI z'Ed(ru)"),
        ("ar", "(en)'Eks w'aI z'Ed(ar)"),
        ("ko", "(en)'Eks w'aI z'Ed(ko)"),
        ("cmn", "(en)'E55ks_| w'aI55_| z'E55d(cmn)_|"),
        ("yue", "(en)'E1ks_| w'aI1_| z'E1d(yue)_|"),
    ];
    for (lang, want) in rows {
        assert_eq!(cli(lang, "x y z"), want, "{lang}");
    }
}
