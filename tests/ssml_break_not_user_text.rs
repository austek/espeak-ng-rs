// tests/ssml_break_not_user_text.rs
//
// An SSML `<break>` is no text: `\x01<digits>` followed by a private-use U+E001 in user text is
// a blanked `\x01` and the characters it leaves, as C's `EmbeddedCommand` reads it (only the
// letters of `PSARHTIVYMUBF` are commands).  Expected values are what C espeak-ng master
// (f0fb11a8) prints for `-x`/`--ipa` and writes for `-w`.

use std::process::Command;

fn cli(args: &[&str]) -> Vec<u8> {
    Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(args)
        .output()
        .expect("run")
        .stdout
}

fn spoken(flag: &str, text: &str) -> String {
    String::from_utf8_lossy(&cli(&["-q", flag, text])).trim().to_string()
}

#[test]
fn a_private_use_letter_after_digits_is_read_as_text() {
    let text = "go \u{01}2000\u{E001} now";
    assert_eq!(spoken("-x", text), "g,oU t'u: T'aUz@nd n'aU");
    assert_eq!(spoken("--ipa", text), "ɡˌəʊ tˈuː θˈaʊzənd nˈaʊ");
}

#[test]
fn a_signed_private_use_letter_after_digits_is_read_as_text() {
    assert_eq!(spoken("-x", "go \u{01}+2000\u{E001} now"), spoken("-x", "go \u{01}+2000 now"));
}

#[test]
fn a_private_use_letter_after_digits_adds_no_pause_to_the_wav() {
    let dir = std::env::temp_dir().join(format!("ssml_break_not_user_text_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let wav = dir.join("out.wav");
    cli(&["-w", wav.to_str().expect("utf-8 path"), "go \u{01}2000\u{E001} now"]);
    let bytes = std::fs::metadata(&wav).expect("wav written").len();
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(bytes, 64040, "C writes 64040 bytes");
}

#[test]
fn an_ssml_break_still_pauses() {
    assert_eq!(
        String::from_utf8_lossy(&cli(&["-q", "-x", "-m", "go <break time=\"2000ms\"/> now"])).trim(),
        "g'oU\nn'aU"
    );
}
