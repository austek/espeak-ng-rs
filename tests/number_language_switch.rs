// A number handed to another voice by the language's `_0lang` entry is a switched word: a
// one-character word is named (C's `FLAG_TRANSLATOR2` spell rule), a longer number starts
// with its own word break, and the switch back closes before the word gap.  Values from
// upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v grc`.

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
fn switched_number_follows_the_c_word_breaks() {
    for (text, want) in [
        ("5", "(en)f'aIv(grc)_:"),
        ("7", "(en)s'Ev@n(grc)_:"),
        ("5 x", "(en)f'aIv(grc)_: ks_:"),
        ("x 5", "ks_: (en)f'aIv(grc)_:"),
        ("5 5", "(en)f'aIv_: f'aIv(grc)_:"),
        ("12", "(en)_: tw'Elv(grc)_:"),
        ("21", "(en)_: tw'Enti_: w'0n(grc)_:"),
        ("12 13", "(en)_: tw'Elv_: T'3:ti:n(grc)_:"),
        ("x 12", "ks_: (en)_: tw'Elv(grc)_:"),
        ("und 12 und", "'ynd_: (en)_: tw'Elv(grc)_: 'ynd_:"),
        ("100", "(en)w'0nh'VndrI2d(grc)_:"),
        ("1000", "(en)w'0n_: T'aUz@nd(grc)_:"),
        ("05", "(en)z'i@roU_: f'aIv(grc)_:"),
    ] {
        assert_eq!(cli("grc", text), want, "grc {text:?}");
    }
}
