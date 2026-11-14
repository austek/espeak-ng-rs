// tests/synth_parity_syllabic.rs
//
// Byte-for-byte WAV parity with C espeak-ng master (f0fb11a8) for the
// syllabic marker `-` after a consonant.  Each expected value is the FNV-1a 64
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
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-sylb-{}-{n}.wav", std::process::id()));
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

/// A syllabic marker makes the preceding consonant a syllable with the
/// stress still pending, and adds no phoneme of its own.
#[test]
fn syllabic_consonants_are_syllables() {
    check(&[
        (&["-v", "ar"], "1234", 61791, 0x20e891fccabb83c4),
        (&["-v", "ar"], "ثلاثة وأربعون", 35914, 0xb92f990bbb5ed001),
        (&["-v", "cs"], "Vlk prst krk", 30487, 0xb16df16159b93ef5),
        (&["-v", "sk"], "Vlk prst krk", 25592, 0xbbc4c50a675b2357),
        (&["-v", "hr"], "Trg prvi", 21241, 0x75ad3ed99383e474),
        (&["-v", "sr"], "Vrt prst", 18254, 0x5891a767473538d1),
        (&["-v", "en"], "bottle button", 22797, 0x79e6392c0118e617),
    ]);
}
