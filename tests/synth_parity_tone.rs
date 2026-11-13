// tests/synth_parity_tone.rs
//
// Tone languages against C espeak-ng master (f0fb11a8): `CalcPitches_Tone`
// gives each syllable the pitch of its tone phoneme.  Expected sample counts
// and FNV-1a hashes of the sample bytes were measured from the C binary with
// `-w`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("espeak-rs-tone-{}-{n}-{name}", std::process::id()))
}

fn render(voice: &str, text: &str) -> Vec<u8> {
    let out = tmp("tone.wav");
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-v", voice, "-w"])
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

/// (voice, text, C sample count, C FNV-1a of the sample bytes)
const CASES: &[(&str, &str, usize, u64)] = &[
    ("vi", "Xin chào", 17923, 0x77817eec0a0cdb9b),
    ("vi", "Tôi là người Việt Nam.", 29956, 0xd38332e2b094c82e),
    ("vi", "Hà Nội", 17912, 0xb95c375164fb3fb5),
    ("cmn", "你好世界", 27860, 0x96d1a4ad4a785f7c),
    ("cmn", "我爱你。", 20348, 0x95abd40f6b3bc3ed),
    ("cmn", "你吃饭了吗?", 30280, 0x6e72f3fc59a9eecd),
    ("yue", "你好嗎", 20834, 0x2ff835e2a86e53db),
    ("hak", "你们好", 61597, 0x722c02340de3f1a4),
];

#[test]
fn tone_language_audio_is_byte_identical_to_c() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    for &(voice, text, samples, hash) in CASES {
        let got = render(voice, text);
        assert_eq!(got.len() / 2, samples, "{voice} {text:?}: sample count");
        assert_eq!(fnv(&got), hash, "{voice} {text:?}: sample bytes");
    }
}
