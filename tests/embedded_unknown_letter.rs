// tests/embedded_unknown_letter.rs
//
// C `ReadClause` blanks the `\x01` of an embedded command and lets `EmbeddedCommand` consume
// the rest only when the letter is one of its commands (translate.c:786-856).  After an unknown
// letter `EmbeddedCommand` returns without advancing, so the sign, digits and letter stay text.
// Expected values are what C espeak-ng master (f0fb11a8) prints for `-x` and writes for `-w`.

use std::process::Command;

fn cli(args: &[&str]) -> Vec<u8> {
    Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(args)
        .output()
        .expect("run")
        .stdout
}

fn phonemes(markup: bool, text: &str) -> String {
    let mut args = vec!["-q", "-x"];
    if markup {
        args.push("-m");
    }
    args.push(text);
    String::from_utf8_lossy(&cli(&args)).trim().to_string()
}

fn wav_bytes(markup: bool, text: &str) -> u64 {
    let dir = std::env::temp_dir().join(format!("embedded_unknown_letter_{}_{markup}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let wav = dir.join("out.wav");
    let path = wav.to_str().expect("utf-8 path");
    let mut args = vec!["-w", path];
    if markup {
        args.push("-m");
    }
    args.push(text);
    cli(&args);
    let bytes = std::fs::metadata(&wav).expect("wav written").len();
    std::fs::remove_dir_all(&dir).ok();
    bytes
}

#[test]
fn an_unknown_command_letter_is_read_as_text() {
    for markup in [false, true] {
        assert_eq!(phonemes(markup, "go\u{01}5Qb"), "g,oU f'aIv kj,u:b'i:", "-m {markup}");
    }
}

#[test]
fn a_sign_before_an_unknown_command_letter_is_read_as_text() {
    assert_eq!(phonemes(false, "go\u{01}+5Qb"), "g,oU pl'Vs f'aIv kj,u:b'i:");
}

#[test]
fn an_unknown_command_letter_without_digits_is_read_as_text() {
    assert_eq!(phonemes(false, "go\u{01}Qb"), "g,oU kj,u:b'i:");
}

#[test]
fn a_known_command_letter_is_still_consumed() {
    assert_eq!(phonemes(false, "go\u{01}50Pb"), phonemes(false, "go b"));
}

#[test]
fn an_unknown_command_letter_makes_the_c_sample_count() {
    for markup in [false, true] {
        assert_eq!(wav_bytes(markup, "go\u{01}5Qb"), 57378, "-m {markup}: C writes 57378 bytes");
    }
}
