// tests/cli_split.rs
//
// `--split=<minutes>` cuts at an output-buffer boundary, not at the sample of
// the SENTENCE event: upstream's `SynthCallback` (espeak-ng.c, f0fb11a8) closes
// the file before the 60 ms buffer holding a SENTENCE event once
// `samples_total > samples_split`.
//
// Expected sample counts are the parts a built upstream `espeak-ng` writes for
// `tests/fixtures/cli_split_text.txt` with `--split=1 -w out.wav`, plain and
// with `-m`.

use std::path::{Path, PathBuf};
use std::process::Command;

const C_PART_SAMPLES: [usize; 2] = [1_353_160, 1_047_085];

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("espeak-rs-split-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn part_samples(path: &Path) -> usize {
    let len = std::fs::metadata(path).expect("part exists").len() as usize;
    (len - 44) / 2
}

fn split_parts(extra: &[&str], name: &str) -> Vec<usize> {
    let dir = tmp_dir(name);
    let text = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli_split_text.txt");
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(extra)
        .arg("-f")
        .arg(&text)
        .arg("--split=1")
        .arg("-w")
        .arg(dir.join("out.wav"))
        .status()
        .expect("run");
    assert!(status.success());
    let mut parts = Vec::new();
    while dir.join(format!("out_{:02}.wav", parts.len() + 1)).exists() {
        parts.push(part_samples(&dir.join(format!("out_{:02}.wav", parts.len() + 1))));
    }
    let _ = std::fs::remove_dir_all(&dir);
    parts
}

macro_rules! skip_without_en {
    () => {
        if !Path::new("espeak-ng-data/en_dict").exists() {
            eprintln!("[SKIP] no en data");
            return;
        }
    };
}

#[test]
fn split_parts_cut_at_c_buffer_boundaries() {
    skip_without_en!();
    assert_eq!(split_parts(&[], "plain"), C_PART_SAMPLES);
}

// Markup delivers one chunk per clause like C, so the 60 ms buffers restart at each clause.
#[test]
fn split_parts_cut_at_c_buffer_boundaries_with_markup() {
    skip_without_en!();
    assert_eq!(split_parts(&["-m"], "markup"), C_PART_SAMPLES);
}
