// tests/synth_parity.rs
//
// The formant synthesizer against C espeak-ng master (f0fb11a8): sample count,
// RMS level and median F0 of the WAV the CLI writes.  Expected values were
// measured from the C binary with the same F0 estimator as `f0_median` below.

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_espeak_cli")
}

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("espeak-rs-sp-{}-{n}-{name}", std::process::id()))
}

fn render(args: &[&str], text: &str) -> Vec<i16> {
    let out = tmp("parity.wav");
    let status = Command::new(bin()).args(args).arg("-w").arg(&out).arg(text).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&out).expect("read wav");
    let _ = std::fs::remove_file(&out);
    bytes[44..].chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect()
}

fn rms(x: &[i16]) -> f64 {
    (x.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / x.len().max(1) as f64).sqrt()
}

/// Median F0 over 40 ms frames every 10 ms: autocorrelation peak between 50
/// and 500 Hz, voiced when the peak exceeds 0.3 of the frame energy.
fn f0_median(x: &[i16]) -> f64 {
    const SR: usize = 22050;
    let (hop, win) = (220, 882);
    let mut f0s = Vec::new();
    let mut i = 0;
    while i + win < x.len() {
        let frame: Vec<f64> = x[i..i + win].iter().map(|&v| v as f64).collect();
        i += hop;
        if (frame.iter().map(|v| v * v).sum::<f64>() / win as f64).sqrt() < 300.0 {
            continue;
        }
        let mean = frame.iter().sum::<f64>() / win as f64;
        let f: Vec<f64> = frame.iter().map(|v| v - mean).collect();
        let ac = |lag: usize| f[..win - lag].iter().zip(&f[lag..]).map(|(a, b)| a * b).sum::<f64>();
        let (lo, hi) = (SR / 500, SR / 50);
        let k = (lo..hi).max_by(|&a, &b| ac(a).total_cmp(&ac(b))).unwrap_or(lo);
        if ac(k) > 0.3 * ac(0) {
            f0s.push(SR as f64 / k as f64);
        }
    }
    f0s.sort_by(f64::total_cmp);
    let n = f0s.len();
    match n {
        0 => 0.0,
        _ if n % 2 == 1 => f0s[n / 2],
        _ => (f0s[n / 2 - 1] + f0s[n / 2]) / 2.0,
    }
}

/// (args, text, C sample count, C RMS, C median F0 in Hz)
const CASES: &[(&[&str], &str, usize, f64, f64)] = &[
    (&["-v", "en"], "hello", 15798, 2769.0, 98.4),
    (&["-v", "en"], "a", 11841, 2073.0, 88.6),
    (&["-v", "en"], "The quick brown fox jumps over the lazy dog.", 61402, 3146.0, 99.8),
    (&["-v", "en"], "Is it raining today? No, it is not.", 57449, 2744.0, 104.5),
    (&["-v", "en", "-s", "250"], "Speaking a little faster now.", 25738, 2935.0, 109.7),
    (&["-v", "en", "-s", "120"], "Speaking slowly.", 40395, 2651.0, 106.0),
    (&["-v", "en", "-p", "80"], "A higher voice.", 23306, 3822.0, 140.0),
    (&["-v", "en", "-a", "40"], "Quietly please.", 26798, 1031.0, 104.0),
    (&["-v", "de"], "Guten Morgen, wie geht es dir?", 41894, 2682.0, 104.5),
    (&["-v", "fr"], "Bonjour tout le monde.", 24398, 2231.0, 106.0),
    (&["-v", "es"], "Hola amigo, buenos dias.", 38924, 2988.0, 109.7),
    (&["-v", "ru"], "Привет, как дела?", 30547, 2588.0, 102.1),
    (&["-v", "ku"], "Rojbaş, tu çawa yî?", 38780, 2965.0, 103.3),
    (&["-v", "ar"], "مرحبا بالعالم", 36180, 2840.0, 107.8),
    (&["-v", "fr"], "yoshi", 16445, 2530.0, 100.7),
    (&["-v", "de"], "enterprise", 21078, 2444.0, 81.1),
    (&["-v", "en"], "Brought it, bought a big boat; then, oddly, sang!", 75868, 2969.0, 98.9),
    (&["-v", "de"], "Ich habe zwanzig Äpfel und dreißig Birnen.", 60124, 2785.0, 97.6),
];

#[test]
fn formant_synthesis_matches_c_length_level_and_pitch() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let mut bad = Vec::new();
    for &(args, text, n, level, f0) in CASES {
        let x = render(args, text);
        let (r, p) = (rms(&x), f0_median(&x));
        let semitones = 12.0 * (p / f0).log2();
        if x.len() != n || (r - level).abs() > level * 0.01 || semitones.abs() > 0.5 {
            bad.push(format!("{args:?} {text:?}: samples {} (C {n}), rms {r:.0} (C {level}), f0 {p:.1} (C {f0})", x.len()));
        }
    }
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
