// The pauses a bracket, quote or emphasis puts before the first word of a clause carry no
// word-start flag (`sourceix == 0` in C's `MakePhonemeList`): the word's first phoneme does, so
// a fricative there gets its 15 ms `prepause`.  Expected values come from
// `espeak-ng -D -v en -w out.wav <text>` (with `-m` for the markup text): the sample count
// ((file bytes - 44) / 2) and the FNV-1a 64 hash of the PCM bytes.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(markup: bool, text: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("clause_leading_pause_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{markup}{:x}.wav", text.len() * 131 + text.bytes().map(usize::from).sum::<usize>()));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.args(["-D", "-v", "en"]);
    if markup {
        cmd.arg("-m");
    }
    let status = cmd.arg("-w").arg(&path).arg(text).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

fn check(markup: bool, cases: &[(&str, usize, u64)]) {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = cases
        .iter()
        .filter_map(|&(text, n, hash)| {
            let pcm = render(markup, text);
            (pcm.len() / 2 != n || fnv1a(&pcm) != hash).then(|| format!("{text:?}: {} samples (C {n})", pcm.len() / 2))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

#[test]
fn bracketed_words_match_c() {
    check(
        false,
        &[
            ("(hello)", 18222, 0x8901ff1f1cfa70fe),
            ("(zoo)", 16825, 0xa8e717eef5b59567),
            ("\"hello\" world", 28290, 0xbcb33615ba2017fd),
            ("[hello]", 18222, 0x8901ff1f1cfa70fe),
            ("(sit down)", 22861, 0x51a57c93f25b1536),
        ],
    );
}

#[test]
fn emphasized_first_word_matches_c() {
    check(
        true,
        &[
            ("<emphasis>hope</emphasis>", 16162, 0xfa0c9fc92f9db1b3),
            ("<emphasis>hope", 16008, 0x1ac2eb378e56f5c3),
        ],
    );
}
