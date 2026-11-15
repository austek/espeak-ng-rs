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

fn lines(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-m", "-v", "en", flag, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// `SelectVoice` scores language, name, gender and variant; `</voice>` pops back to the
/// base voice unless `base_voice->languages` (the last voice loaded) is another voice's.
#[test]
fn voice_selection_matches_c() {
    check(&[
        ("hello <voice gender=\"female\">world</voice> there", 32951, 0x177da4f7a677883b),
        ("hello <voice name=\"de\">welt</voice> there", 31229, 0xb9419a380e80856f),
        ("hello <voice name=\"en-us\">world</voice> there", 31028, 0x141dbd94c4eb56e0),
        ("hello <voice xml:lang=\"es\" gender=\"male\">mundo</voice> there", 30866, 0xc247b37b7902f727),
        ("hello <voice variant=\"2\">world</voice> there", 31631, 0x8097fc55bae9e3aa),
        (
            "hello <voice xml:lang=\"en-us\">world</voice> there <voice xml:lang=\"de\">welt</voice> there",
            45603,
            0xef00291b4fb1ae86,
        ),
    ]);
}

#[test]
fn voice_switch_lines_match_c() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        return;
    }
    let text = "hello <voice xml:lang=\"en-us\">world</voice> there <voice xml:lang=\"de\">welt</voice> there";
    assert_eq!(lines("--ipa", text), "həlˈəʊ\nwˈɜːld ðˈɛɹ\nvˈɛlt\nðˈeə\n");
    assert_eq!(lines("-x", text), "h@l'oU\nw'3:ld D'e@\nv'Elt\nD'e@\n");
    let female = "hello <voice gender=\"female\">world</voice> there";
    assert_eq!(lines("--ipa", female), "həlˈəʊ\nwˈɜːld\nðˈeə\n");
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
