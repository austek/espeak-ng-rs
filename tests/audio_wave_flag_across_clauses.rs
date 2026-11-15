// `DoSpect2`'s `static int wave_flag` outlives a clause: the first formant sequence of the
// next clause cancels a wave still playing (`WCMD_SPECT2`).  Expected values come from
// `espeak-ng -D -v ar -w out.wav <text>`: the sample count ((file bytes - 44) / 2) and the
// FNV-1a 64 hash of the PCM bytes.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(text: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("audio_wave_flag_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", text.len() * 131 + text.bytes().map(usize::from).sum::<usize>()));
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-D", "-v", "ar", "-w"])
        .arg(&path)
        .arg(text)
        .status()
        .expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

#[test]
fn arabic_sentences_match_c_sample_for_sample() {
    if !std::path::Path::new("espeak-ng-data/ar_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases: [(&str, usize, u64); 3] = [
        ("أربعة. خمسة.", 32002, 0x67bb2c721a2d382c),
        ("واحد. اثنان. ثلاثة. أربعة. خمسة.", 89864, 0x7cd3f2b9ca12b95c),
        ("مرحبا. كيف حالك؟ أنا بخير.", 72002, 0x12f67968c550004c),
    ];
    let bad: Vec<String> = cases
        .iter()
        .filter_map(|&(text, n, hash)| {
            let pcm = render(text);
            (pcm.len() / 2 != n || fnv1a(&pcm) != hash).then(|| format!("{text:?}: {} samples (C {n})", pcm.len() / 2))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
