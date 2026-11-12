// tests/synth_parity_voice_path.rs
//
// A `-v` value that names a voice file (`-v kw/fx` for `voices/kw/fx`) speaks
// with that file's keywords, as C's `LoadVoice` reads them.  The voice files
// below are written into a scratch data directory that links the repository
// data; the expected values come from C espeak-ng master (f0fb11a8) given the
// same files in a copy of its data directory, with `-D`.  Each is the FNV-1a
// 64 hash of the PCM samples plus their count.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

const VOICES: &[(&str, &str)] = &[
    ("samp", "name kwsamp\nlanguage en\nstressAmp 10 12 30 30 20 22 28 25\n"),
    ("fmt", "name kwfmt\nlanguage en\nformant 1 110 90 120 50\nformant 2 90 110 80\nformant 0 105 95 110\n"),
    ("fx", "name kwfx\nlanguage en\necho 120 25\nflutter 5\nroughness 4\nvoicing 80\nconsonants 60 70\n"),
    (
        "br",
        "name kwbr\nlanguage en\nbreath 10 20 30 10 5 0 0 0\nbreathw 150 300 500 500 500\ntone 200 150 1000 220 3000 100 6000 90\n",
    ),
    ("ps2", "name kwps2\nlanguage en\npitch 100 160\nspeed 120\nintonation 2\n"),
];

const TEXT: &str = "The quick brown fox jumps over the lazy dog.";

/// (voice, C sample count, C PCM hash)
const CASES: &[(&str, usize, u64)] = &[
    ("kw/samp", 61402, 0xb436c7114db2146b),
    ("kw/fmt", 61402, 0x25bdf7850898d0ce),
    ("kw/fx", 61855, 0x9e54753aab2657b3),
    ("kw/br", 61402, 0x18fe0842c16f2688),
    ("kw/ps2", 51240, 0x07aa1a0aafb9bc5c),
];

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn link_entries(from: &Path, to: &Path, skip: &str) {
    std::fs::create_dir_all(to).expect("mkdir");
    for e in std::fs::read_dir(from).expect("read data").flatten() {
        if e.file_name() != skip {
            std::os::unix::fs::symlink(e.path(), to.join(e.file_name())).expect("symlink");
        }
    }
}

fn scratch_data() -> PathBuf {
    let root = std::env::temp_dir().join(format!("espeak-rs-vpath-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("espeak-ng-data");
    let ours = root.join("espeak-ng-data");
    link_entries(&data, &ours, "voices");
    link_entries(&data.join("voices"), &ours.join("voices"), "");
    let kw = ours.join("voices").join("kw");
    std::fs::create_dir_all(&kw).expect("mkdir kw");
    VOICES.iter().for_each(|(n, body)| std::fs::write(kw.join(n), body).expect("write voice"));
    root
}

fn render(root: &Path, voice: &str) -> Vec<u8> {
    let out = root.join(format!("{}.wav", voice.replace('/', "_")));
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .arg(format!("--path={}", root.display()))
        .args(["-D", "-v", voice, "-w"])
        .arg(&out)
        .arg(TEXT)
        .status()
        .expect("run");
    assert!(status.success());
    std::fs::read(&out).expect("read wav")[44..].to_vec()
}

#[test]
fn voice_named_by_path_matches_c_byte_for_byte() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let root = scratch_data();
    let bad: Vec<String> = CASES
        .iter()
        .filter_map(|&(voice, n, hash)| {
            let pcm = render(&root, voice);
            (pcm.len() / 2 != n || fnv1a(&pcm) != hash).then(|| format!("{voice}: {} samples (C {n})", pcm.len() / 2))
        })
        .collect();
    let _ = std::fs::remove_dir_all(&root);
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
