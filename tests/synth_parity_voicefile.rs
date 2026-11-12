// tests/synth_parity_voicefile.rs
//
// Byte-for-byte WAV parity with C espeak-ng master (f0fb11a8) for languages
// whose voice file sets synthesis keywords (`stressAmp`, `stressLength`,
// `stressAdd`, `intonation`, `pitch`, `formant`, `speed`), which C's
// `LoadVoice` applies over `SelectTranslator`.  Each expected value is the
// FNV-1a 64 hash of the PCM samples the C binary writes, plus its count.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-spvf-{}-{n}.wav", std::process::id()));
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
    (&["-v", "fi"], "Kissa nukkuu sohvalla olohuoneessa.", 55262, 0x5a7b8e2347a1ec46),
    (&["-v", "fi"], "Missä rautatieasema on?", 37662, 0x11999cd65ea881ad),
    (&["-v", "hr"], "Volim piti kavu s mlijekom.", 41740, 0x5e308dcbb6006ae8),
    (&["-v", "hr"], "Gdje je željeznički kolodvor?", 43175, 0xb52b9ccf5f8e849b),
    (&["-v", "ta"], "பூனை சோபாவில் தூங்குகிறது.", 46561, 0x4ed64a39bcb927e6),
    (&["-v", "sr"], "Мачка спава на каучу.", 36261, 0xeb31179181afc55c),
    (&["-v", "bs"], "Mačka spava na kauču u sobi.", 44145, 0x888c36728005bd98),
    (&["-v", "sv"], "Jag dricker gärna kaffe med mjölk.", 34062, 0xb4fae2cb67a6b8e8),
    (&["-v", "uk"], "Кіт спить на дивані.", 45012, 0x66e76256863db32c),
];

#[test]
fn voice_file_keywords_match_c_byte_for_byte() {
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
