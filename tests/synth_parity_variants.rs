// tests/synth_parity_variants.rs
//
// Byte-for-byte WAV parity with C espeak-ng master (f0fb11a8) for `+variant`
// voices.  C splits the variant off a voice name before lowercasing it, so
// mixed-case variant files (`!v/AnxiousAndy`) load.  Each expected value is
// the FNV-1a 64 hash of the PCM samples the C binary writes, plus its count.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-spvr-{}-{n}.wav", std::process::id()));
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
fn mixed_case_variants_match_c_byte_for_byte() {
    check(&[
        (&["-v", "de+AnxiousAndy"], "Das Wetter ist heute schön.", 42387, 0xb659e400b265d106),
        (&["-v", "en+AnxiousAndy"], "The weather is lovely today.", 42317, 0x848f8a056b079ceb),
        (&["-v", "de+Storm"], "Das Wetter ist heute schön.", 36999, 0x9355658f78129a9a),
        (&["-v", "fr+Andy"], "Il fait beau aujourd'hui.", 29685, 0x2c57084bf3d2a0e5),
        (&["-v", "es+Henrique"], "Hace buen tiempo hoy.", 31641, 0x2d20c912fd4ff14b),
    ]);
}
