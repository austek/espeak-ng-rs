// tests/synth_parity_next3.rs
//
// Phoneme programs that test `next3PhW` and `prev2PhW` (Danish vowel lengths,
// Hindi and Bengali conditions) against C espeak-ng master (f0fb11a8): sample
// counts and FNV-1a hashes of the sample bytes were measured from the C binary
// with `-w`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("espeak-rs-next3-{}-{n}-{name}", std::process::id()))
}

fn render(voice: &str, text: &str) -> Vec<u8> {
    let out = tmp("next3.wav");
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
    ("da", "aften", 13664, 0xd44f18e4d2b8eecc),
    ("da", "byttet", 13557, 0xc859d3cfbc2cdea5),
    ("da", "himlen", 15770, 0x4c9701a13ef39bda),
    ("da", "sigte", 14995, 0x94646acfc364c10b),
    ("da", "Jeg hedder Peter", 26312, 0xf051c36bc608b3e0),
    ("hi", "नमस्ते दुनिया", 24627, 0x69de629c6e9f9e21),
    ("bn", "আমি বাংলায় কথা বলি", 37228, 0x5de11f51a46d4fe5),
];

#[test]
fn next3_and_prev2_conditions_match_c_audio() {
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
