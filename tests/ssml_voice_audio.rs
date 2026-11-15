// `-D -m` audio of clauses read by another voice, against C espeak-ng master (f0fb11a8).
// Expected values come from `espeak-ng -D -m -v en -w out.wav <text>`: the sample count
// ((file bytes - 44) / 2) and the FNV-1a 64 hash of the PCM bytes.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(text: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("ssml_voice_audio_{}", std::process::id()));
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

fn check(cases: &[(&str, usize, u64)]) {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = cases
        .iter()
        .filter_map(|&(text, n, hash)| {
            let pcm = render(text);
            (pcm.len() / 2 != n || fnv1a(&pcm) != hash).then(|| format!("{text:?}: {} samples (C {n})", pcm.len() / 2))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

/// The clause after a voice change uses that voice's pitch, formants and tables.
#[test]
fn language_voice_changes_the_generator_voice() {
    check(&[
        ("hello <voice xml:lang=\"fr\">bonjour</voice> world", 32990, 0x8baa76e366c519a3),
        ("<speak xml:lang=\"fr\">bonjour</speak>", 16136, 0x1817102309ca1f25),
        ("<voice xml:lang=\"de\">hallo</voice> hello", 22954, 0x6beb4b27cd24b82b),
        ("hello <p xml:lang=\"de\">welt</p> there", 56539, 0x8b766d0aa609d82e),
    ]);
}
