// A word spoken through a `_^_` language switch takes its inter-word pause from the base voice's
// `MakePhonemeList`, not from the switched language: af `amico` is Latin, and Latin's own word
// gap must not add a pause before the switch back.  Expected values come from
// `espeak-ng -v af -w out.wav <text>` of espeak-ng f0fb11a8: the sample count
// ((file bytes - 44) / 2) and the FNV-1a 64 hash of the PCM bytes.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(lang: &str, text: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("switched_word_gap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{lang}_{:x}.wav", text.bytes().map(usize::from).sum::<usize>()));
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-v", lang, "-w"])
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
fn afrikaans_latin_words_match_c_sample_for_sample() {
    if !std::path::Path::new("espeak-ng-data/af_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases: [(&str, usize, u64); 7] = [
        ("amico", 15547, 0xdb5838f4de4bfd98),
        ("fide", 15025, 0x0dad757089dff7d1),
        ("sine", 14527, 0x3dca55674ffc3ba8),
        ("persona", 19238, 0x6c36c308dc557081),
        ("factum", 18623, 0xeeeba011825a5abf),
        ("fide sine hond", 28215, 0x3c9372dc884a8969),
        ("ek fide.", 18749, 0xd2e6a1aee90f5c88),
    ];
    let bad: Vec<String> = cases
        .iter()
        .filter_map(|&(text, n, hash)| {
            let pcm = render("af", text);
            (pcm.len() / 2 != n || fnv1a(&pcm) != hash).then(|| format!("{text:?}: {} samples (C {n})", pcm.len() / 2))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
