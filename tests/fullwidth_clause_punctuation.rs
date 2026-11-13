// Full-width `，` `！` `？` `；` `：` end a clause on their own, without a
// following space, as the CJK full stop does; they are not rewritten to the
// ASCII marks, which need a space after them. Expected mnemonics are from C
// espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(lang: &str, text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("cmn_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', "/"))
}

#[test]
fn fullwidth_marks_break_the_clause() {
    let want = "ni35X'Au214_|/s.'i.51_| tS;'iE51_|";
    for mark in ["，", "！", "？", "；", "：", "、", "。"] {
        let text = format!("你好{mark}世界");
        let Some(got) = mnemonics("cmn", &text) else {
            eprintln!("[SKIP] no data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}

#[test]
fn ascii_comma_without_space_does_not_break() {
    let Some(got) = mnemonics("cmn", "你好,世界。") else {
        eprintln!("[SKIP] no data");
        return;
    };
    assert_eq!(got, "ni35X'Au21_| s.'i.51_| tS;'iE51_|");
}

#[test]
fn fullwidth_comma_in_english_text_breaks_the_clause() {
    let Some(got) = mnemonics("en", "a，b") else {
        eprintln!("[SKIP] no data");
        return;
    };
    assert_eq!(got, "'eI/b'i:");
}
