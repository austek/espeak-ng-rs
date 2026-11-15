// `-D -m` audio of `<prosody>` text, against C espeak-ng master (f0fb11a8).  Expected values
// come from `espeak-ng -D -m -v en -w out.wav <text>`: the sample count ((file bytes - 44) / 2)
// and the FNV-1a 64 hash of the PCM bytes.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(text: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("ssml_prosody_audio_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", text.len() * 131 + text.bytes().map(usize::from).sum::<usize>()));
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-D", "-m", "-v", "en", "-w"])
        .arg(&path)
        .arg(text)
        .status()
        .expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

/// Commands after the last word (`</prosody>` writes the rate back) run before the clause
/// pause, so the pause keeps the surrounding rate.
#[test]
fn prosody_around_the_whole_text_matches_c() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases: [(&str, usize, u64); 7] = [
        ("<prosody rate=\"x-slow\">hello world</prosody>", 33376, 0x7ed835484f0bd880),
        ("<prosody rate=\"slow\">hello world</prosody>", 26632, 0xbaf74d7585fe15c2),
        ("<prosody rate=\"fast\">hello world</prosody>", 19754, 0xb2dd20c64d470a90),
        ("<prosody rate=\"150%\">hello world</prosody>", 17311, 0xa3a9631844a6d6c8),
        ("<prosody rate=\"120\">hello world</prosody>", 12759, 0x33c379a4dd51e615),
        ("<prosody pitch=\"low\" rate=\"slow\">hello world</prosody>", 27211, 0x605bd3a75e97606c),
        ("<prosody rate=\"slow\">one <prosody rate=\"fast\">two</prosody> three</prosody>", 25051, 0xd66314613f9906aa),
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
