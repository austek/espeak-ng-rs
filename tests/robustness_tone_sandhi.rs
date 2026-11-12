// Fuzz crash: two 214 syllables inside one entry used to shift the tone slot
// index and panic. Tail values from upstream espeak-ng master (f0fb11a8),
// `espeak-ng -q -x -v cmn "ﾙ闹"`:
//   @:11_|e11f_|e11f_|tS;'iou35_|tS;'iou21_:_| n'Au51_|

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(args)
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli")
}

#[test]
fn double_third_tone_inside_one_entry_does_not_panic() {
    let out = run(&["-q", "-x", "-v", "cmn", "--", "\u{FF99}\u{95F9}"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|");
    assert!(
        text.ends_with("tS;'iou35_|tS;'iou21_:_| n'Au51_|"),
        "{text}"
    );
}

#[test]
fn utf16_fuzz_crash_input_exits_cleanly() {
    let path = std::env::temp_dir().join("robustness_tone_sandhi_crash.bin");
    std::fs::write(&path, [0x99u8, 0xff, 0xf9, 0x95]).expect("write fixture");
    let out = run(&["-q", "-x", "-b", "4", "-v", "cmn", "-f", path.to_str().expect("utf8")]);
    let _ = std::fs::remove_file(&path);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}
