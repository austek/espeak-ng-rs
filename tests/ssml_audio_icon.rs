// `<audio src>` with a loadable WAV plays the clip as C's `EMBED_I` does and silences the
// text inside the tag.  Expected values come from `espeak-ng -D -m -v en -w out.wav <text>` on
// the same 5500-sample mono 16-bit 22050 Hz sawtooth file: the sample count ((file bytes - 44)
// / 2) and the FNV-1a 64 hash of the PCM bytes.

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

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(dir: &std::path::Path, text: &str) -> Vec<u8> {
    let out = dir.join(format!("{:x}.wav", text.len() * 131 + text.bytes().map(usize::from).sum::<usize>()));
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-D", "-m", "-v", "en", "-w"])
        .arg(&out)
        .arg(text)
        .status()
        .expect("run");
    assert!(status.success());
    std::fs::read(&out).expect("wav")[44..].to_vec()
}

#[test]
fn audio_tag_plays_the_clip_like_c() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let dir = std::env::temp_dir().join(format!("ssml_audio_icon_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let icon = dir.join("icon.wav");
    std::fs::write(&icon, icon_wav()).expect("icon");
    let src = icon.to_string_lossy().into_owned();
    let cases: [(&str, usize, u64); 7] = [
        ("x <audio src=\"/nonexistent/nope.wav\">fallback text</audio> y", 39730, 0x88886cf1424ff969),
        ("start <audio src=\"ICON\">beep</audio> end", 26315, 0xdc80f98c518241dd),
        ("<audio src=\"ICON\"/>", 5808, 0xbe6467486730c4b4),
        ("<audio src=\"ICON\">beep</audio>", 5808, 0xbe6467486730c4b4),
        ("hello <audio src=\"ICON\">x</audio> world <audio src=\"ICON\">y</audio> again", 42158, 0x7c91db72cdce1fdd),
        ("<speak>hi <audio src=\"ICON\"/></speak>", 18475, 0xf5b35ae425d738ed),
        ("one. <audio src=\"ICON\"/> two.", 24888, 0xa9aa7bef0690d36d),
    ];
    let bad: Vec<String> = cases
        .iter()
        .filter_map(|&(text, n, hash)| {
            let pcm = render(&dir, &text.replace("ICON", &src));
            (pcm.len() / 2 != n || fnv1a(&pcm) != hash).then(|| format!("{text:?}: {} samples (C {n})", pcm.len() / 2))
        })
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
