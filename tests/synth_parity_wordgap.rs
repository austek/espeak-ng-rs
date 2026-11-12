// tests/synth_parity_wordgap.rs
//
// Byte-for-byte WAV parity with C espeak-ng master (f0fb11a8) for `-g`, the
// pause `MakePhonemeList` inserts between words (`option_wordgap * 14`).
// Each expected value is the FNV-1a 64 hash of the PCM samples the C binary
// writes, plus its sample count.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-spwg-{}-{n}.wav", std::process::id()));
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

/// (args, text, C sample count, C PCM hash)
const CASES: &[(&[&str], &str, usize, u64)] = &[
    (&["-v", "en", "-g", "10"], "The quick brown fox jumps.", 54106, 0x5d9bf18beadf4a54),
    (&["-v", "en", "-g", "1"], "Hello there my friend.", 31801, 0xd0046323c0d0a9df),
    (&["-v", "de", "-g", "5"], "Guten Morgen, wie geht es?", 43952, 0xca1447efa4f3db3f),
    (&["-v", "fr", "-g", "20"], "Je suis très content.", 46402, 0x8c3ab9debb64f4b2),
    (&["-v", "es", "-g", "8"], "El perro come carne.", 39040, 0xf522b4de4a68072e),
];

#[test]
fn word_gap_matches_c_byte_for_byte() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .filter_map(|&(args, text, n, hash)| {
            let pcm = render(args, text);
            (pcm.len() / 2 != n || fnv1a(&pcm) != hash)
                .then(|| format!("{args:?} {text:?}: {} samples (C {n})", pcm.len() / 2))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
