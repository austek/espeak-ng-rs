// A spelled word whose first letters are silent opens with its pause directly,
// with no word separator after it. Expected strings are espeak-ng master (f0fb11a8).

use std::process::Command;

fn spoken(text: &str) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .env("ESPEAK_DATA_PATH", env!("CARGO_MANIFEST_DIR"))
        .args(["-q", "-v", "shn", "-x", "--", text])
        .output()
        .expect("spawn");
    assert_eq!(o.status.code(), Some(0));
    String::from_utf8_lossy(&o.stdout).trim_end().to_string()
}

#[test]
fn silent_leading_letter_leaves_no_separator_after_the_pause() {
    assert_eq!(spoken("ၵꩡ"), "_:(en)mj'A:1nmA@1(shn)k");
    assert_eq!(spoken("ꩠၵ"), "_:(en)mj'A:1nmA@1(shn)k");
    assert_eq!(spoken("ꩢမ"), "_:(en)mj'A:1nmA@1(shn)m");
}
