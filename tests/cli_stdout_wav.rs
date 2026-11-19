// tests/cli_stdout_wav.rs
//
// `--stdout` and `-w stdout` write the bytes of the reference C `espeak-ng` (f0fb11a8): the
// header carries the placeholder sizes 0x7ffff024 and 0x7ffff000 and is never patched, even
// when stdout is a regular file.  `tests/fixtures/cli_stdout_wav.tsv` holds, per row,
// `args<TAB>text<TAB>length<TAB>fnv64 of all bytes<TAB>header hex`, written by
// `record_from_c` below from that binary.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_espeak_cli")
}

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli_stdout_wav.tsv")
}

const CASES: &[(&str, &str)] = &[
    ("", ""),
    ("", "Hello"),
    ("-v de", "Guten Morgen."),
    ("-s 120", "One. Two. Three."),
];

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn row(args: &str, text: &str, out: &[u8]) -> String {
    format!("{args}\t{text}\t{}\t{:016x}\t{}\n", out.len(), fnv(out), hex(&out[..44.min(out.len())]))
}

fn tmp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("espeak-rs-{}-{name}", std::process::id()))
}

fn run(exe: &str, data: Option<&str>, args: &str, text: &str, stdout_args: &[&str], to_file: bool) -> Vec<u8> {
    let mut cmd = Command::new(exe);
    if let Some(d) = data {
        cmd.env("ESPEAK_DATA_PATH", d);
    }
    cmd.args(args.split_whitespace()).args(stdout_args).arg(text);
    if to_file {
        let path = tmp("redirect.wav");
        cmd.stdout(std::fs::File::create(&path).expect("create"));
        assert!(cmd.status().expect("run").success());
        let bytes = std::fs::read(&path).expect("read");
        let _ = std::fs::remove_file(&path);
        bytes
    } else {
        cmd.stdout(Stdio::piped());
        cmd.output().expect("run").stdout
    }
}

#[test]
fn stdout_wav_equals_the_c_bytes() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no en data");
        return;
    }
    let recorded = std::fs::read_to_string(fixture_path()).expect("fixture");
    let mut failures = Vec::new();
    for (args, text) in CASES {
        let want = recorded.lines().find(|l| l.starts_with(&format!("{args}\t{text}\t"))).expect("row");
        for (mode, flags, to_file) in [
            ("pipe", &["--stdout"][..], false),
            ("redirected file", &["--stdout"][..], true),
            ("-w stdout", &["-w", "stdout"][..], true),
        ] {
            let got = run(bin(), None, args, text, flags, to_file);
            if row(args, text, &got).trim_end() != want {
                failures.push(format!("{mode} {args:?} {text:?}: {}", hex(&got[..44.min(got.len())])));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Records the fixture from the reference binary: `C_ESPEAK_BIN` is its path and
/// `C_ESPEAK_DATA` the directory holding its `espeak-ng-data`.  Run with `--ignored`.
#[test]
#[ignore]
fn record_from_c() {
    let exe = std::env::var("C_ESPEAK_BIN").expect("C_ESPEAK_BIN");
    let data = std::env::var("C_ESPEAK_DATA").expect("C_ESPEAK_DATA");
    let rows: String = CASES
        .iter()
        .map(|(args, text)| row(args, text, &run(&exe, Some(&data), args, text, &["--stdout"], false)))
        .collect();
    std::fs::write(fixture_path(), rows).expect("write fixture");
}
