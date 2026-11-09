// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v it`.

fn check_phonemes(cases: &[(&str, &str)]) {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    for (text, want) in cases {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
            .args(["-q", "-x", "-v", "it", "--", text])
            .env("ESPEAK_DATA_PATH", data)
            .output()
            .expect("run cli");
        let got = String::from_utf8_lossy(&out.stdout).trim().replace('\n', " ");
        assert_eq!(&got, want, "text {text:?}");
    }
}

#[test]
fn end_stress_is_followed_by_the_alternate_vowel() {
    check_phonemes(&[("del", "d'el"), ("dal", "d'al")]);
}

#[test]
fn word_entry_flags_win_over_the_stem_flags() {
    check_phonemes(&[
        ("privilegia", "p@-*ivil'edZa"),
        ("ciascune", "tSask'une"),
        ("agite", "adZ'ite"),
        ("sedilo", "s'Edilo"),
    ]);
}
