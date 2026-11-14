// A Cyrillic voice that has no case of its own in `tr_languages.c` runs with
// `letter_bits_offset` 0: a Latin word has no rule there, so it is spelled
// letter by letter instead of being handed to English whole.
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
fn a_latin_word_is_spelled_letter_by_letter() {
    let rows = [
        ("ky", "кафеbar", "qaf'e _:(en)b'i: _:'eI _:'A@(ky)"),
        ("ky", "3km", "'ytS _:(en)k,eI_:'Em(ky)"),
        ("ky", "7th", "dZet'i _:(en)t,i:_:'eItS(ky)"),
        ("ky", "50kg", "el'y: _:(en)k,eI_:dZ'i:(ky)"),
        ("ky", "text", "_:(en)t'i: _:'i: _:'Eks _:t'i:(ky)"),
        ("ba", "bar", "_:(en)b'i: _:'eI _:'A@(ba)"),
        ("cv", "bar", "_:(en)b'i: _:'eI _:'A@(cv)"),
    ];
    for (lang, text, want) in rows {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}
