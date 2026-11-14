// tests/synth_parity_echo_exact_fill.rs
//
// Byte-for-byte WAV parity with C espeak-ng master (f0fb11a8) when speech ends
// exactly on an output buffer boundary: C's fill loop leaves the zero-length
// commands that close the clause for the next buffer, and the echo tail plays
// there.  Values are the FNV-1a 64 hash of the PCM samples C writes, plus the
// sample count.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-echofill-{}-{n}.wav", std::process::id()));
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(args)
        .arg("-w")
        .arg(&out)
        .arg(text)
        .status()
        .expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&out).expect("read wav");
    let _ = std::fs::remove_file(&out);
    bytes[44..].to_vec()
}

type Case = (&'static [&'static str], &'static str, usize, u64);

fn check(cases: &[Case]) {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = cases
        .iter()
        .filter_map(|&(args, text, n, hash)| {
            let pcm = render(args, text);
            (pcm.len() / 2 != n || fnv1a(&pcm) != hash)
                .then(|| format!("{args:?} {text:?}: {} samples (C {n})", pcm.len() / 2))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

#[test]
fn the_echo_tail_follows_speech_that_ends_on_a_buffer_boundary() {
    check(&[
        (&["-v", "en+announcer"], "And another one.", 23985, 0xbf7a61c2cc94a1f3),
        (&["-v", "en+announcer"], "Hello world, this is a test. And another one.", 66353, 0xc3c16cf183904680),
        (&["-v", "en+announcer"], "Hello. And another one.", 39873, 0x2a51f07b249fe011),
    ]);
}
