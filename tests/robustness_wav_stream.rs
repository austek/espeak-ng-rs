// WAV output is written clause by clause: memory stays flat however long the
// text is, the file header is patched at the end, and a pipe gets upstream's
// placeholder sizes.

use std::process::{Command, Stdio};

const SENTENCES: &str = "The quick brown fox jumps over the lazy dog, then 42 cats sat quietly. \
Where did everyone go? Nobody knows! ";

fn data() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data")
}

fn input(name: &str, bytes: usize) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("robustness_wav_{name}.txt"));
    std::fs::write(&path, SENTENCES.repeat(bytes / SENTENCES.len() + 1)).expect("write input");
    path
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

#[test]
fn file_header_sizes_match_the_audio() {
    let text = input("header", 2_000);
    let wav = std::env::temp_dir().join("robustness_wav_header.wav");
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-w", wav.to_str().expect("utf8"), "-f", text.to_str().expect("utf8")])
        .env("ESPEAK_DATA_PATH", data())
        .status()
        .expect("run cli");
    assert!(status.success());
    let bytes = std::fs::read(&wav).expect("read wav");
    let _ = std::fs::remove_file(&wav);
    assert_eq!(u32_at(&bytes, 40) as usize, bytes.len() - 44);
    assert_eq!(u32_at(&bytes, 4) as usize, bytes.len() - 8);
}

#[test]
fn piped_stdout_gets_upstream_placeholder_sizes() {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["--stdout", "hello"])
        .env("ESPEAK_DATA_PATH", data())
        .output()
        .expect("run cli");
    assert!(out.status.success());
    assert_eq!(u32_at(&out.stdout, 4), 0x7fff_ff24);
    assert_eq!(u32_at(&out.stdout, 40), 0x7fff_f000);
    assert!(out.stdout.len() > 44);
}

#[cfg(unix)]
#[test]
fn long_text_wav_fits_in_a_small_address_space() {
    let text = input("memory", 100_000);
    let status = Command::new("sh")
        .args(["-c", "ulimit -v 250000; exec \"$0\" --stdout -f \"$1\""])
        .arg(env!("CARGO_BIN_EXE_espeak_cli"))
        .arg(&text)
        .env("ESPEAK_DATA_PATH", data())
        .stdout(Stdio::null())
        .status()
        .expect("run cli");
    assert!(status.success(), "{status:?}");
}
