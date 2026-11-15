// A clause that ends on the dot of an abbreviation (`$allowdot`/`$needsdot` word last, dot
// after the last word) gets a 10 ms pause and no tone (translate.c: `clause_pause = 10;
// *tone_out = 4`).  Expected values come from `espeak-ng -D -v en -w out.wav <text>`, with and
// without `-m`: the sample count ((file bytes - 44) / 2) and the FNV-1a 64 hash of the PCM bytes.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(markup: bool, text: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("abbreviation_clause_pause_{}", std::process::id()));
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

fn check(markup: bool) {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases: [(&str, usize, u64); 4] = [
        ("Dr. Who", 21811, 0x7388e5d055596a08),
        ("Dr. Smith lives on Main St.", 46161, 0xb00599140ed45596),
        ("Mr. Jones is here. Mrs. Smith too.", 60800, 0x89eaf9c17b69015e),
        ("I met Prof. Brown at 5 p.m. today.", 53703, 0x8aa77d8980d987dc),
    ];
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
fn plain_text_abbreviation_clause_matches_c() {
    check(false);
}

#[test]
fn markup_abbreviation_clause_matches_c() {
    check(true);
}
