// tests/synth_parity_punct_pause.rs
//
// Pause and intonation of the clause terminators beyond `, ; : ? !` (ellipsis,
// dashes, inverted marks, Arabic and ideographic marks) against C espeak-ng
// master (f0fb11a8): sample counts and FNV-1a hashes of the sample bytes were
// measured from the C binary with `-w`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("espeak-rs-ppause-{}-{n}-{name}", std::process::id()))
}

fn render(text: &str) -> Vec<u8> {
    let out = tmp("punct.wav");
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-v", "en", "-w"])
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

/// (text, C sample count, C FNV-1a of the sample bytes)
const CASES: &[(&str, usize, u64)] = &[
    ("hello … world", 29907, 0x757ec7514c05e13c),
    ("hello — world", 29907, 0x757ec7514c05e13c),
    ("hello ¿ world", 29907, 0x757ec7514c05e13c),
    ("hello ؟ world", 31612, 0x91a6552ecea2335f),
    ("hello ، world", 28231, 0x25a7528930304fbc),
    ("hello 、 world", 28231, 0x25a7528930304fbc),
    ("hello ... ", 14190, 0xdf0d9f9a4dbdc4fc),
    ("The end...", 14408, 0xb7ffe07e8fcd81fd),
];

#[test]
fn clause_terminator_pauses_match_c() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    for &(text, samples, hash) in CASES {
        let got = render(text);
        assert_eq!(got.len() / 2, samples, "{text:?}: sample count");
        assert_eq!(fnv(&got), hash, "{text:?}: sample bytes");
    }
}
