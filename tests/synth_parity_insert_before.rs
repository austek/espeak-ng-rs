// tests/synth_parity_insert_before.rs
//
// Byte-for-byte WAV parity with C espeak-ng master (f0fb11a8) for a phoneme a
// program inserts before another one.  Each expected value is the FNV-1a 64
// hash of the PCM samples the C binary writes, plus its sample count.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-insb-{}-{n}.wav", std::process::id()));
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

/// The inserted vowel replaces its neighbour in C's list, unstressed when
/// that was a consonant; the neighbour follows with no stress or flags.
#[test]
fn an_inserted_vowel_is_unstressed_before_a_consonant() {
    check(&[
        (&["-v", "ru"], "гражданство", 21188, 0xee2e7c6301f260b0),
        (&["-v", "ru"], "гражданине", 19305, 0xb84a439770622a14),
        (&["-v", "ru"], "гражданстве", 22327, 0xf8c384f0c58ca3e2),
        (&["-v", "ru"], "Добрый вечер", 21211, 0xf7787e2b36712070),
    ]);
}
