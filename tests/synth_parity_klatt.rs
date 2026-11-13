// tests/synth_parity_klatt.rs
//
// Klatt voices (`klatt 1` to `klatt 5` in a voice file) against C espeak-ng
// master (f0fb11a8): sample counts and FNV-1a hashes of the sample bytes were
// measured from the C binary with `-w`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("espeak-rs-klatt-{}-{n}-{name}", std::process::id()))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    let out = tmp("klatt.wav");
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
    (&["-v", "en+klatt"], "hello world", 22674, 0xe42586f16e325ad9),
    (&["-v", "en+klatt"], "The quick brown fox jumps over the lazy dog.", 61441, 0xb0ec0da69e4ad044),
    (&["-v", "en+klatt3"], "Is it raining today? No, it is not.", 59628, 0x32251f025b23f00e),
    (&["-v", "de+klatt2"], "Guten Morgen, wie geht es dir?", 42029, 0x4d29c3c18e24408a),
    (&["-v", "fr+klatt4"], "Bonjour tout le monde.", 25257, 0xb858cb046caa1480),
    (&["-v", "es+klatt5"], "Hola amigo, buenos dias.", 40021, 0x30ba878745ad6cb8),
    (&["-v", "ru+klatt"], "Привет, как дела?", 30731, 0x6ec488c36f802f04),
    (&["-v", "en+edward"], "Speaking a little faster now.", 39882, 0x3b5390a00c8233e4),
    (&["-v", "en+klatt", "-s", "250"], "Hello there", 12789, 0x39958ec8c07036b2),
    (&["-v", "en+klatt2", "-p", "80"], "A higher voice.", 23061, 0xb3ecfcec03670fc0),
    (&["-v", "ar+klatt"], "مرحبا بالعالم", 35953, 0x154f517f1a390df2),
];

#[test]
fn klatt_voice_audio_is_byte_identical_to_c() {
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
