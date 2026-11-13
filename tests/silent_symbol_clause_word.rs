// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <lang>`.
// A guillemet the language names only as `_»` is silent in running text but still
// counts as a word of the clause, so the word before it is not clause-final.

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
fn guillemet_after_the_last_word_demotes_its_stress() {
    assert_eq!(cli("fr", "« Merci beaucoup »"), "_:_:mErs'i bok,u");
    assert_eq!(cli("fr", "Merci beaucoup»"), "mErs'i bok,u");
    assert_eq!(cli("fr", "Merci beaucoup «"), "mErs'i bok,u");
}

#[test]
fn languages_without_the_name_entry_keep_the_stress() {
    assert_eq!(cli("it", "Merci beaucoup »"), "m'E@-*tSI beaUk'oUp");
}
