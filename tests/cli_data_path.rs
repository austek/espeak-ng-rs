// `ESPEAK_DATA_PATH` and `--path` name the directory that contains `espeak-ng-data`, as in C; the data directory itself is accepted too.

use std::path::Path;
use std::process::Command;

const HELLO_IPA: &str = "həlˈəʊ wˈɜːld\n";

fn parent() -> &'static str {
    env!("CARGO_MANIFEST_DIR")
}

fn run(env_path: Option<&str>, args: &[&str]) -> (Option<i32>, String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.env_remove("ESPEAK_DATA_PATH").env("HOME", "/nonexistent").args(args);
    if let Some(p) = env_path {
        cmd.env("ESPEAK_DATA_PATH", p);
    }
    let out = cmd.output().expect("spawn");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn have_data() -> bool {
    Path::new(parent()).join("espeak-ng-data/en_dict").exists()
}

#[test]
fn env_names_the_parent_of_the_data_directory() {
    if !have_data() {
        return;
    }
    let (code, out, err) = run(Some(parent()), &["-q", "--ipa", "Hello world"]);
    assert_eq!((code, out.as_str(), err.as_str()), (Some(0), HELLO_IPA, ""));
}

#[test]
fn env_naming_the_data_directory_itself_still_works() {
    if !have_data() {
        return;
    }
    let data = format!("{}/espeak-ng-data", parent());
    let (code, out, _) = run(Some(&data), &["-q", "--ipa", "Hello world"]);
    assert_eq!((code, out.as_str()), (Some(0), HELLO_IPA));
}

#[test]
fn path_option_names_the_parent_of_the_data_directory() {
    if !have_data() {
        return;
    }
    let arg = format!("--path={}", parent());
    let (code, out, _) = run(None, &["-q", "--ipa", &arg, "Hello world"]);
    assert_eq!((code, out.as_str()), (Some(0), HELLO_IPA));
}

#[test]
fn unusable_path_option_falls_back_to_the_environment() {
    if !have_data() {
        return;
    }
    let (code, out, _) =
        run(Some(parent()), &["-q", "--ipa", "--path=/nonexistent", "Hello world"]);
    assert_eq!((code, out.as_str()), (Some(0), HELLO_IPA));
}

#[test]
fn version_reports_the_resolved_data_directory() {
    if !have_data() {
        return;
    }
    let (_, out, _) = run(Some(parent()), &["--version"]);
    assert!(out.trim_end().ends_with(&format!("Data at: {}/espeak-ng-data", parent())), "{out}");
}
