// tests/synth_parity_penultimate.rs
//
// A voice file `intonation` value with the penultimate-emphasis bit (the
// robosoft2 variant's `intonation -25`) puts the tonic on the penultimate
// stressed syllable.  Expected values come from C espeak-ng master (f0fb11a8)
// with `-w`: sample counts and FNV-1a hashes of the sample bytes.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("espeak-rs-penult-{}-{n}-{name}", std::process::id()))
}

fn render(voice: &str, text: &str) -> Vec<u8> {
    let out = tmp("penult.wav");
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-v", voice, "-w"])
        .arg(&out)
        .arg(text)
        .status()
        .expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&out).expect("read wav");
    let _ = std::fs::remove_file(&out);
    bytes[44..].to_vec()
}

fn fnv(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf29ce484222325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

/// (voice, text, C sample count, C FNV-1a of the sample bytes)
const CASES: &[(&str, &str, usize, u64)] = &[
    ("en+robosoft2", "Robots can talk.", 25542, 0x41445f50ab1c1ce3),
    ("en+robosoft2", "Robots can talk. Really?", 37988, 0x52047c03fa12436d),
    ("de+robosoft2", "Guten Morgen, wie geht es dir?", 36528, 0x087ed9fe4e2f5c57),
];

#[test]
fn penultimate_emphasis_matches_c_audio() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    for &(voice, text, samples, hash) in CASES {
        let got = render(voice, text);
        assert_eq!(got.len() / 2, samples, "{voice} {text:?}: sample count");
        assert_eq!(fnv(&got), hash, "{voice} {text:?}: sample bytes");
    }
}
