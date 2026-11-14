// Only the languages `SelectTranslator` gives a `letter_bits_offset` read their
// rules through letter groups keyed on a script block. `ba` and `kok` have no
// case there, so their rules see the Latin letter groups. Values from upstream
// espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <lang>`.

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
fn voices_without_a_translator_case_use_latin_letter_groups() {
    let rows = [
        ("ba", "аи", "'0i"),
        ("ba", "ауыл", "0'uVL"),
        ("kok", "का", "k,@a:k'a:r"),
        ("kok", "ok", "_:(en)'oU _:k'eI(kok)"),
        ("kok", "quiz", "_:(en)kj'u: _:j'u: _:'aI _:z'Ed(kok)"),
    ];
    for (lang, text, want) in rows {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}
