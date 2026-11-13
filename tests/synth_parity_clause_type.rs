// tests/synth_parity_clause_type.rs
//
// Pause and intonation of every clause terminator C classifies through
// `clause_type_from_codepoint` (full-width, Devanagari, Ethiopic, Armenian
// marks and others) against C espeak-ng master (f0fb11a8): sample counts and
// FNV-1a hashes of the sample bytes were measured from the C binary with `-w`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("espeak-rs-ctype-{}-{n}-{name}", std::process::id()))
}

fn render(lang: &str, text: &str) -> Vec<u8> {
    let out = tmp("clause.wav");
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-v", lang, "-w"])
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
    ("en", "hello， world", 28231, 0x25a7528930304fbc),
    ("en", "hello！", 16785, 0x81d8743c124f4dee),
    ("en", "hello？", 15895, 0xcce255fa98ce6640),
    ("en", "hello； world", 29907, 0x757ec7514c05e13c),
    ("en", "hello： world", 29860, 0x8007677493708107),
    ("en", "hello‼", 16785, 0x81d8743c124f4dee),
    ("en", "hello ‼ world", 32502, 0xc01e2e460c055a6a),
    ("hi", "नमस्ते ॥", 22151, 0x461c30987bf50143),
    ("hi", "नमस्ते ॥ दुनिया", 36912, 0xa501272a5c4cb6c6),
    ("am", "ሰላም ፣ ዓለም", 29916, 0x947bbea84490fbfb),
    ("am", "ሰላም ፧", 16816, 0x23f993e8a88e71f6),
    ("hy", "բարև ՝ աշխարհ", 32416, 0x915ffa3c57d850e6),
    ("cmn", "你好，世界", 32822, 0xba57b0c5c18466d0),
    ("cmn", "你好！", 18571, 0x1d2d593162ee07ba),
    ("cmn", "你好：世界", 34498, 0x1f07ab14c2b40510),
];

#[test]
fn clause_type_pauses_match_c() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    for &(lang, text, samples, hash) in CASES {
        let got = render(lang, text);
        assert_eq!(got.len() / 2, samples, "{lang} {text:?}: sample count");
        assert_eq!(fnv(&got), hash, "{lang} {text:?}: sample bytes");
    }
}
