// tests/synth_parity_echo_tail.rs
//
// Byte-for-byte WAV parity with C espeak-ng master (f0fb11a8) for the echo of
// a voice, which C plays out whenever its command queue runs empty, that is
// after every clause.  Each expected value is the FNV-1a 64 hash of the PCM
// samples the C binary writes, plus its sample count.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-echo-{}-{n}.wav", std::process::id()));
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
fn the_echo_plays_out_after_every_clause() {
    check(&[
        (&["-v", "en+announcer"], "Hello, world. How are you?", 46704, 0xc911154b34a1b612),
        (&["-v", "en+announcer"], "One. Two. Three.", 41129, 0xfbfe9d00236a33b6),
        (&["-v", "en+announcer"], "Stop! Go? Yes, no.", 53055, 0xace0586b073abdb1),
        (&["-v", "en"], "Hello, world. How are you?", 45623, 0x9ec735cbfc682c77),
    ]);
}
