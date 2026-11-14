// A word is broken where its script changes, and each run is read on its own;
// a spelled word whose first letters are silent opens with its pause directly.
// Expected strings are espeak-ng master (f0fb11a8).

use std::process::Command;

fn spoken(lang: &str, flag: &str, text: &str) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .env("ESPEAK_DATA_PATH", env!("CARGO_MANIFEST_DIR"))
        .args(["-q", "-v", lang, flag, "--", text])
        .output()
        .expect("spawn");
    assert_eq!(o.status.code(), Some(0));
    String::from_utf8_lossy(&o.stdout).trim_end().to_string()
}

#[test]
fn mixed_script_words_are_read_in_runs() {
    let cases = [
        ("ky", "кгkм", "kilˌoɡrɑm (en)kˈeɪ(ky) m"),
        ("ru", "приветhello", "prʲivʲˈet(en)həlˈəʊ(ru)"),
    ];
    for (lang, text, want) in cases {
        assert_eq!(spoken(lang, "--ipa", text), want, "{lang} {text}");
    }
}
