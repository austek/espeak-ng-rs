// Messages, exit statuses and side effects of the command line match C: --version, --help, error texts, -q, --phonout, stdin lines.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_espeak_cli");
const DATA_PARENT: &str = env!("CARGO_MANIFEST_DIR");

struct Run {
    code: Option<i32>,
    out: String,
    err: String,
}

fn run_with(args: &[&str], stdin: &[u8]) -> Run {
    let mut child = Command::new(BIN)
        .env("ESPEAK_DATA_PATH", DATA_PARENT)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child.stdin.take().expect("stdin").write_all(stdin).expect("write");
    let o = child.wait_with_output().expect("wait");
    Run {
        code: o.status.code(),
        out: String::from_utf8_lossy(&o.stdout).into_owned(),
        err: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

fn run(args: &[&str]) -> Run {
    run_with(args, b"")
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

#[test]
fn version_is_the_c_line() {
    let r = run(&["--version"]);
    let want = format!("eSpeak NG text-to-speech: 1.53.0  Data at: {DATA_PARENT}/espeak-ng-data\n");
    assert_eq!((r.code, r.out, r.err), (Some(0), want, String::new()));
}

#[test]
fn help_is_the_c_text_after_a_blank_line_and_the_version_line() {
    let r = run(&["--help"]);
    let version = format!("eSpeak NG text-to-speech: 1.53.0  Data at: {DATA_PARENT}/espeak-ng-data");
    let want = format!("\n{version}\n{}", include_str!("cli_help/body.txt"));
    assert_eq!((r.code, r.out, r.err), (Some(0), want.clone(), String::new()));
    assert_eq!(run(&["-h"]).out, want);
}

#[test]
fn unknown_voice_prints_the_c_error_and_exits_1() {
    let r = run(&["-q", "-x", "-v", "nonexistent", "hi"]);
    let want = "Error: The specified espeak-ng voice does not exist.\n";
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (Some(1), "", want));
}

#[test]
fn missing_input_file_is_reported_with_status_1() {
    let r = run(&["-q", "-f", "missing.txt"]);
    assert_eq!((r.code, r.err.as_str()), (Some(1), "Failed to stat() file 'missing.txt'\n"));
}

#[test]
fn unwritable_wav_is_reported_with_status_0() {
    let r = run(&["-w", "/nonexistent/x.wav", "hi"]);
    assert_eq!((r.code, r.err.as_str()), (Some(0), "Can't write to: '/nonexistent/x.wav'\n"));
}

#[test]
fn compile_without_a_voice_name_is_reported() {
    let r = run(&["--compile"]);
    assert_eq!((r.code, r.err.as_str()), (Some(1), "Voice name to '--compile' not specified.\n"));
}

#[test]
fn quiet_pho_prints_nothing() {
    let r = run(&["-q", "--pho", "Hello"]);
    assert_eq!((r.code, r.out.as_str()), (Some(0), ""));
}

#[test]
fn quiet_stdout_writes_no_wav() {
    let r = run(&["--stdout", "-q", "hi"]);
    assert_eq!((r.code, r.out.as_str()), (Some(0), ""));
}

#[test]
fn quiet_wav_file_is_never_created() {
    let wav = scratch("cli_messages_quiet").join("q.wav");
    run(&["-q", "-w", wav.to_str().expect("utf8"), "hi"]);
    assert!(!wav.exists());
}

#[test]
fn phonout_file_exists_even_when_nothing_is_written() {
    let file = scratch("cli_messages_phonout").join("po.txt");
    let r = run(&["-q", &format!("--phonout={}", file.display()), "hi"]);
    assert_eq!(r.code, Some(0));
    assert_eq!(std::fs::read(&file).expect("created").len(), 0);
}

#[test]
fn speech_without_an_output_option_fails_like_a_missing_audio_device() {
    let r = run(&["hi"]);
    let want = "Error: Cannot initialize the audio device.\n";
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (Some(1), "", want));
}

#[test]
fn compile_phonemes_missing_source_prints_the_c_messages() {
    let r = run(&["--compile-phonemes=/nonexistent"]);
    assert_eq!(r.out, "Compiling phoneme data: /nonexistent/phonemes\n");
    assert_eq!(
        (r.code, r.err.as_str()),
        (Some(1), "Error processing file '/nonexistent/phonemes': No such file or directory.\n")
    );
}

#[test]
fn stdin_lines_are_separate_synth_calls() {
    let r = run_with(&["-q", "-x"], b"foo bar\nbaz qux\n");
    assert_eq!(r.out, "f'u: b'A@\nb'az kw@ks\n");
}

#[test]
/// Guards against regression; the behaviour already held before the messages fix.
fn bulk_stdin_is_one_synth_call() {
    let r = run_with(&["-q", "-x", "--stdin"], b"foo bar\nbaz qux\n");
    assert_eq!(r.out, "f'u: b'A@ b'az kw@ks\n");
}

#[test]
fn empty_stdin_prints_nothing() {
    let r = run(&["-q", "-x"]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (Some(0), "", ""));
}
