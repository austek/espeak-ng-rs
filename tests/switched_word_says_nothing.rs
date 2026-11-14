// A word handed to another voice that has no sound for it stays silent, also
// when another such word follows. Values from upstream espeak-ng master
// (f0fb11a8), `espeak-ng -q -x -v <lang>`: every row prints nothing.

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
fn consecutive_words_the_target_voice_cannot_say_stay_silent() {
    let rows = [
        ("en", "\u{ab73} \u{ab73}"),
        ("en", "\u{ab70} \u{ab70}"),
        ("ko", "\u{ab73} \u{ab73}"),
        ("ko", "\u{ab73} \u{ab73} \u{ab73} \u{ab73}"),
        ("kk", "\u{ab73} \u{abad}"),
        ("de", "\u{ab73} \u{ab73}"),
    ];
    for (lang, text) in rows {
        assert_eq!(cli(lang, text), "", "{lang} {text:?}");
    }
}
