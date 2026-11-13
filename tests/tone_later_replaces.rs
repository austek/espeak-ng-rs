// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v <lang>` and `-q -x -v <lang>`.

fn cli(lang: &str, flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

fn cases(lang: &str, flag: &str, rows: &[(&str, &str)]) {
    for (text, want) in rows {
        assert_eq!(&cli(lang, flag, text), want, "{lang} {flag} {text:?}");
    }
}

#[test]
fn burmese_visarga_replaces_the_vowels_own_tone() {
    cases("my", "-x", &[("သီး", "D'i2"), ("ဖရဲသီး", "phj'aDi2")]);
    cases("my", "--ipa", &[("သီး", "ðˈi2"), ("ဖရဲသီး", "phjˈaði2")]);
}

#[test]
fn thai_tone_mark_after_the_vowel_wins() {
    cases(
        "th",
        "-x",
        &[
            ("ระหว่าง", "r'a5sha2w sng"),
            ("หัวใจสีแดง", "hmw,ai3cha5ssa'e3da2ng"),
            ("สิงคโปร์", "s,i2nga2kh'o3pa2r"),
        ],
    );
}
