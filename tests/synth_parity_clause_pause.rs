// tests/synth_parity_clause_pause.rs
//
// The pause of a clause that has no words, and of the last clause of the text,
// against C espeak-ng master (f0fb11a8): sample counts and FNV-1a hashes of
// the sample bytes were measured from the C binary with `-w`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("espeak-rs-cpause-{}-{n}-{name}", std::process::id()))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    let out = tmp("pause.wav");
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
    (&["-v", "en"], ".", 154, 0xc10fa6af0a66b7b5),
    (&["-v", "en"], "hello . .", 15952, 0x0a594482a775c19e),
    (&["-v", "en"], "hello . . .", 15952, 0x0a594482a775c19e),
    (&["-v", "en"], "hello , .", 12668, 0xf86c498e05bf413c),
    (&["-v", "en"], "hello ; .", 14344, 0x5d7abc389b92bfbc),
    (&["-v", "en"], "hello ! .", 16939, 0x527b099dcffe8e4e),
    (&["-v", "de"], "Guten Tag . .", 22224, 0x9eaf785c4e7723a1),
    (&["-v", "en", "-z"], "hello world", 16192, 0x3589dad743071683),
    (&["-v", "en", "-z"], "hello.", 9315, 0x964da1f12970872e),
];

#[test]
fn empty_and_final_clause_pauses_match_c() {
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
