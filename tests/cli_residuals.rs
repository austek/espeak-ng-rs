// tests/cli_residuals.rs
//
// CLI residuals of issue #21 against C espeak-ng f0fb11a8: `-X` prints the trace
// per clause, `--split` runs one synthesis per stdin line and reports an
// unwritable target like `-w` does, and `synth_streaming_texts` always ends on a
// final chunk.
//
// Expected values come from a C build of f0fb11a8: FNV-1a 64 over whole files
// or streams, and the exact text it prints.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_espeak_cli")
}

fn have_data() -> bool {
    Path::new("espeak-ng-data/en_dict").exists()
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn scratch_dir(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("espeak-rs-res-{}-{n}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run_with_stdin(args: &[&str], stdin: &str) -> Output {
    use std::io::Write;
    let mut child = Command::new(bin())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    child.wait_with_output().expect("wait")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("utf-8")
}

const HELLO_TRACE: &str = "Found: 'hello' [h@loU]  \nh@l'oU\n";

#[test]
fn trace_prints_per_clause_with_the_audio_on_stdout() {
    if !have_data() {
        return;
    }
    for args in [&["-X", "-w", "stdout", "Hello. Hello."][..], &["-X", "-x", "-w", "stdout", "Hello. Hello."]] {
        let out = run_with_stdin(args, "").stdout;
        assert_eq!((out.len(), fnv(&out)), (63338, 0xfc5c_bf61_38df_fef2), "{args:?}");
        assert_eq!(&out[..32], HELLO_TRACE.as_bytes());
        assert_eq!(&out[32..36], b"RIFF");
        assert_eq!(&out[31672..31672 + 32], HELLO_TRACE.as_bytes());
    }
    let out = run_with_stdin(&["-X", "-w", "stdout"], "hello\nhello\n").stdout;
    assert_eq!((out.len(), fnv(&out)), (63338, 0xfc5c_bf61_38df_fef2));
    let out = run_with_stdin(&["-X", "-w", "stdout"], "hello\n\nhello\n").stdout;
    assert_eq!((out.len(), fnv(&out)), (63647, 0x9aa1_2bef_e6f0_b358));
}

#[test]
fn trace_prints_per_clause_without_audio() {
    if !have_data() {
        return;
    }
    let twice = HELLO_TRACE.repeat(2);
    for args in [&["-X", "-q", "Hello. Hello."][..], &["-X", "-x", "-q", "Hello. Hello."]] {
        assert_eq!(text(&run_with_stdin(args, "").stdout), twice, "{args:?}");
    }
    let ipa = "Found: 'hello' [h@loU]  \nhəlˈəʊ\n".repeat(2);
    assert_eq!(text(&run_with_stdin(&["-X", "--ipa", "-q", "Hello. Hello."], "").stdout), ipa);
    assert_eq!(text(&run_with_stdin(&["-X", "-q"], "hello\nhello\n").stdout), twice);
}

const SPLIT_LINES: &str = "hello world\ngood morning\n";

fn split_run(extra: &[&str], stdin: &str, target: &Path) -> (Output, Vec<u8>) {
    let mut args = vec!["-w", target.to_str().unwrap(), "--split=1"];
    args.extend(extra);
    let out = run_with_stdin(&args, stdin);
    let stem = target.with_extension("");
    let first = PathBuf::from(format!("{}_01.wav", stem.display()));
    let bytes = std::fs::read(first).unwrap_or_default();
    (out, bytes)
}

#[test]
fn split_runs_one_synthesis_per_stdin_line() {
    if !have_data() {
        return;
    }
    let dir = scratch_dir("split");
    let (out, wav) = split_run(&[], SPLIT_LINES, &dir.join("out.wav"));
    assert!(out.status.success());
    assert_eq!((wav.len(), fnv(&wav)), (87420, 0x754c_9cd4_5b70_2492));
    let (out, wav) = split_run(&["-x"], SPLIT_LINES, &dir.join("x.wav"));
    assert_eq!(text(&out.stdout), "h@l'oU w'3:ld\ng'Ud m'O@nIN\n");
    assert_eq!((wav.len(), fnv(&wav)), (87420, 0x754c_9cd4_5b70_2492));
    let (_, wav) = split_run(&[], "hello world\ngood morning\nthank you\n", &dir.join("three.wav"));
    assert_eq!((wav.len(), fnv(&wav)), (125868, 0x4063_6405_f69a_6b8d));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn split_to_an_unwritable_target_reports_once_per_synthesis_and_exits_zero() {
    if !have_data() {
        return;
    }
    let out = run_with_stdin(&["-w", "/nonexistent/o.wav", "--split=1"], SPLIT_LINES);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(text(&out.stdout), "");
    assert_eq!(text(&out.stderr), "Can't write to: '/nonexistent/o_01.wav'\nCan't write to: '/nonexistent/o_01.wav'\n");

    let out = run_with_stdin(&["-x", "-w", "/nonexistent/o.wav", "--split=1"], SPLIT_LINES);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(text(&out.stdout), "h@l'oU w'3:ld\ng'Ud m'O@nIN\n");
    assert_eq!(text(&out.stderr), "Can't write to: '/nonexistent/o_01.wav'\nCan't write to: '/nonexistent/o_01.wav'\n");

    let out = run_with_stdin(&["-w", "/nonexistent/o.wav", "--split=1", "hi. there"], "");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(text(&out.stderr), "Can't write to: '/nonexistent/o_01.wav'\n");

    let out = run_with_stdin(&["-w", "/nonexistent/o", "--split=1", "hi"], "");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(text(&out.stderr), "Can't write to: '/nonexistent/o_01'\n");
}

#[test]
fn streaming_texts_ends_on_one_final_chunk_for_empty_input() {
    if !have_data() {
        return;
    }
    let data = Path::new("espeak-ng-data");
    let plain = espeak_ng::EspeakNg::with_data_dir("en", data).expect("engine");
    let slowed = espeak_ng::EspeakNg::with_data_dir("en", data).expect("engine").with_post_tempo(1.5);
    for engine in [plain, slowed] {
        let mut calls = Vec::new();
        let none: [&str; 0] = [];
        engine
            .synth_streaming_texts(&none, |chunk, last| {
                calls.push((chunk.len(), last));
                false
            })
            .expect("synth");
        assert_eq!(calls, vec![(0, true)]);
    }
}
