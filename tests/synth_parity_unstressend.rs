// tests/synth_parity_unstressend.rs
//
// A word marked `$unstressend` loses its stress at the end of a clause, which
// changes the intonation and lengths of the audio but not the phoneme string.
// Expected values come from C espeak-ng master (f0fb11a8) with `-w`: sample
// counts and FNV-1a hashes of the sample bytes.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("espeak-rs-unstressend-{}-{n}-{name}", std::process::id()))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    let out = tmp("unstress.wav");
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

fn fnv(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf29ce484222325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

/// (args, text, C sample count, C FNV-1a of the sample bytes)
const CASES: &[(&[&str], &str, usize, u64)] = &[
    (&["-v", "en"], "one", 13894, 0x9e2b0f147eb08c3e),
    (&["-v", "en"], "one,", 10564, 0xb55fba275539af5e),
    (&["-v", "en"], "one, two, three", 34400, 0xf6f5c7f17d9584de),
    (&["-v", "en"], "it is the one", 22921, 0xdeca721cf4dc0fcb),
    (&["-v", "en"], "just one", 18636, 0x2f5b0ffa23f1a3ec),
    (&["-v", "en", "-z"], "one, two, three", 27917, 0x9fd385b9cbbca8ce),
];

#[test]
fn unstressend_words_lose_stress_at_clause_end_like_c() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    for &(args, text, samples, hash) in CASES {
        let got = render(args, text);
        assert_eq!(got.len() / 2, samples, "{args:?} {text:?}: sample count");
        assert_eq!(fnv(&got), hash, "{args:?} {text:?}: sample bytes");
    }
}
