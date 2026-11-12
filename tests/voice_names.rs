// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <voice>`.

fn run(voice: &str, text: &str) -> (String, String) {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", voice, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    (
        String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|"),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn voice_file_path_under_lang_selects_that_voice() {
    assert_eq!(run("gmw/en", "hello world").0, "h@l'oU w'3:ld");
    assert_eq!(run("roa/fr", "hello world").0, "El'o (en)w'3:ld(fr)");
    assert_eq!(run("gmw/en+f2", "hello world").0, "h@l'oU w'3:ld");
}

#[test]
fn dialect_voice_files_keep_their_phoneme_table() {
    let text = "hello world, the dogs";
    for (voice, want) in [
        ("gmw/en-GB-scotland", "h@l'oU w'VRld|D@ d'0gz"),
        ("gmw/en-029", "h@l'oU w'3:ld|d@2 d'0gz"),
        ("gmw/en-US", "h@l'oU w'3:ld|D@2 d'0gz"),
        ("gmw/en-GB-x-rp", "h@l'oU w'3:ld|D@2 d'0gz"),
    ] {
        assert_eq!(run(voice, text).0, want, "{voice}");
    }
}

#[test]
fn numeric_and_unknown_variants_are_silent() {
    for v in ["en+3", "en+13", "en+zzz", "en+f2+croak"] {
        let (out, err) = run(v, "hello world");
        assert_eq!(out, "h@l'oU w'3:ld", "{v}");
        assert_eq!(err, "", "{v}");
    }
}

#[test]
fn numeric_variant_names_map_to_m_and_f_files() {
    use espeak_ng::voices::variant_file_name;
    assert_eq!(variant_file_name("3"), "m3");
    assert_eq!(variant_file_name("13"), "f3");
    assert_eq!(variant_file_name("0"), "0");
    assert_eq!(variant_file_name("croak"), "croak");
}
