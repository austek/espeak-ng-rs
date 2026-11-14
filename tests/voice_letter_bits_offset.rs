// `SelectTranslator` in `tr_languages.c` has no `hyw` case, so Western
// Armenian runs with `letter_bits_offset` 0 although it shares the `hy`
// dictionary: a Latin word has no rule there and is spelled in English.
// Languages without a translator case of their own (`ba`, `kok`) likewise use
// the Latin letter groups. Values from upstream espeak-ng master (f0fb11a8),
// `espeak-ng -q -x -v <lang>`.

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
fn western_armenian_spells_latin_words_by_letter() {
    let rows = [
        ("hyw", "zap", "_:(en)z'Ed(hy) 'a _:(en)p'i:(hy)"),
        ("hyw", "text", "_:(en)t'i:(hy) 'e _:(en)'Eks _:t'i:(hy)"),
        ("hyw", "Wolf", "_:(en)d'Vb@Lj,u: _:'oU _:'El(hy) 'ef"),
        ("hyw", "bcd", "p#'i s'i t#'i"),
    ];
    for (lang, text, want) in rows {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}
