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
fn a_suffix_after_a_digit_and_apostrophe_is_a_word_of_its_own() {
    for (lang, text, want) in [
        ("ku", "2024'an", "d'U hEz'ar _u b'Ist_utS'ar 'an"),
        ("ku", "1990'an", "hEz'ar _un'Ehs'Ed_u n'od 'an"),
        ("en", "2024'an", "t'u: T'aUz@nd @n tw'Enti f'o@r- 'an"),
        ("tr", "2024'an", "ic'i b'in jiRm,id'WRt 'an"),
    ] {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}
