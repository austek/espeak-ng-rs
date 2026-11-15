// tests/ssml_audio_phoneme_output.rs
//
// `<audio src>` with a loadable WAV silences the alternative text in the phoneme output too
// (`--ipa`, `-x`), not only in the audio.  Expected values are what C espeak-ng master
// (f0fb11a8) prints for the same file.

use std::process::Command;

fn icon_wav() -> Vec<u8> {
    let n = 5500usize;
    let mut w = Vec::new();
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&((36 + n * 2) as u32).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&22_050u32.to_le_bytes());
    w.extend_from_slice(&44_100u32.to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&((n * 2) as u32).to_le_bytes());
    (0..n).for_each(|i| w.extend_from_slice(&((((i % 100) as i16) - 50) * 200).to_le_bytes()));
    w
}

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn audio_alternative_text_is_not_printed() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let dir = std::env::temp_dir().join(format!("ssml_audio_phoneme_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let wav = dir.join("clip.wav");
    std::fs::write(&wav, icon_wav()).expect("write wav");
    let text = format!("<speak><audio src=\"{}\">alt</audio> done.</speak>", wav.display());
    let ipa = run("--ipa", &text);
    let mnemonics = run("-x", &text);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(ipa, "\n\ndˈʌn\n\n", "--ipa");
    assert_eq!(mnemonics, "\n\nd'Vn\n\n", "-x");
}
