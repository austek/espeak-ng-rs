// tests/synth_parity_klatt6.rs
//
// Byte-for-byte WAV parity with C espeak-ng master (f0fb11a8) for the `klatt 6`
// voice, which synthesizes with the NV Speech Player generator.  Each expected
// value is the FNV-1a 64 hash of the PCM samples the C binary writes, plus its
// sample count.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-k6-{}-{n}.wav", std::process::id()));
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
fn klatt6_voices_use_the_speech_player() {
    check(&[
        (&["-v", "en+klatt6"], "Hello world.", 22518, 0x7a87871b52e0cb20),
        (&["-v", "en+klatt6"], "Hello, world. How are you?", 45428, 0xed5edfbd6d2c8b0e),
        (&["-v", "de+klatt6"], "Guten Tag, wie geht es dir?", 42387, 0xd6475a74484bcb18),
        (&["-v", "es+klatt6"], "Buenos días", 22948, 0xa595b17431c8fb76),
        (&["-v", "ru+klatt6"], "Добрый вечер", 21199, 0x298569ee14cdf81a),
        (&["-v", "en+klatt6", "-s", "120", "-p", "60"], "The quick brown fox.", 47211, 0x35583ec1ab010632),
        (&["-v", "en+klatt6"], "zebra vision", 22434, 0x3aa924f0481413e5),
    ]);
}
