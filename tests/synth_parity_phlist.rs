// tests/synth_parity_phlist.rs
//
// Byte-for-byte WAV parity with C espeak-ng master (f0fb11a8) for the stress
// and word-start marks `MakePhonemeList` leaves on the phoneme list.  Each
// expected value is the FNV-1a 64 hash of the PCM samples the C binary
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
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-spph-{}-{n}.wav", std::process::id()));
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

/// An unstressed vowel ending a word keeps its stress when the clause's last
/// word is stressed, whatever its own word's stress.
#[test]
fn alternate_unstressed_vowels_follow_the_last_words_stress() {
    check(&[
        (&["-v", "nl"], "De kat slaapt op de bank.", 37448, 0xe0fa75a167ec0da8),
        (&["-v", "sv"], "Katten sover på soffan i vardagsrummet.", 38172, 0xe5670fe4992cd891),
        (&["-v", "hr"], "Kuća je vrlo velika.", 33044, 0xd7bce60ed74468c7),
        (&["-v", "bs"], "Kuća je velika.", 26692, 0x851e345f0757d7ce),
    ]);
}
