// tests/synth_parity_deleted_next.rs
//
// Byte-for-byte WAV parity with C espeak-ng master (f0fb11a8) for the
// alternate-syllable stress reduction when a phoneme that a program deletes
// (German word-final `r` after `@`) still follows the vowel in C's list.  Values
// are the FNV-1a 64 hash of the PCM samples C writes, plus the sample count.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-delnext-{}-{n}.wav", std::process::id()));
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
fn a_deleted_phoneme_after_a_vowel_is_not_a_word_start() {
    check(&[
        (&["-v", "de"], "Das ist ein langsamer Satz.", 34038, 0x0be5140c91604136),
        (&["-v", "de"], "langsamer Satz", 23323, 0xfea1cdf924833731),
        (&["-v", "de"], "Das Wasser ist kalt.", 28511, 0x163ccc4d4873dada),
        (&["-v", "de"], "Der Lehrer liest ein Buch.", 33249, 0xe61feeab4a6025bb),
    ]);
}
