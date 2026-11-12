// `-v` selects voices as C does (name, file path, language) and fails with C's message otherwise; start-up checks and phoneme-output flag combinations follow C.

use std::process::Command;

const NO_VOICE: &str = "Error: The specified espeak-ng voice does not exist.\n";

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    run_in(env!("CARGO_MANIFEST_DIR"), args)
}

fn run_in(data: &str, args: &[&str]) -> (Option<i32>, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .env("ESPEAK_DATA_PATH", data)
        .args(args)
        .output()
        .expect("spawn");
    (
        o.status.code(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn rejects_voice(voice: &str) {
    assert_eq!(run(&["-q", "-x", "-v", voice, "hello"]), (Some(1), String::new(), NO_VOICE.into()));
}

fn speaks(args: &[&str], phonemes: &str) {
    assert_eq!(run(args), (Some(0), phonemes.to_string(), String::new()));
}

#[test]
fn voice_names_are_matched_exactly_ignoring_case() {
    speaks(&["-q", "-x", "-v", "French (France)", "bonjour"], "bO~Z'ur\n");
    rejects_voice("French");
    rejects_voice("french_(france)");
}

#[test]
fn underscore_is_not_a_hyphen_in_language_tags() {
    rejects_voice("en_gb");
    speaks(&["-q", "-x", "-v", "EN-GB", "hello"], "h@l'oU\n");
}

#[test]
fn file_path_and_language_selectors() {
    speaks(&["-q", "-x", "-v", "roa/fr", "bonjour"], "bO~Z'ur\n");
    speaks(&["-q", "-x", "-v", "pt-br", "ola"], "'Ol&\n");
}

#[test]
fn empty_voice_name_means_the_default_voice() {
    speaks(&["-q", "-x", "-v", "", "hello"], "h@l'oU\n");
}

#[test]
fn missing_variant_file_is_not_reported() {
    speaks(&["-q", "-x", "-v", "en+nonexistentvariant", "hello"], "h@l'oU\n");
}

#[test]
fn mbrola_voice_prints_its_language_phonemes() {
    speaks(&["-q", "-x", "-v", "mb-en1", "hello"], "h@l'oU\n");
}

#[test]
fn compile_checks_the_voice_first() {
    assert_eq!(run(&["--compile=xx"]), (Some(1), String::new(), NO_VOICE.into()));
}

#[test]
fn data_directory_without_phontab_is_reported() {
    let want = "Error processing file '/tmp/phontab': No such file or directory.\n";
    assert_eq!(run_in("/tmp", &["-q", "-x", "hello"]), (Some(1), String::new(), want.into()));
}

#[test]
fn pho_does_not_replace_x_output() {
    speaks(&["-q", "--pho", "-x", "hello"], "h@l'oU\n");
}

#[test]
fn input_encoding_applies_to_the_text_argument() {
    speaks(&["-q", "-x", "-b", "4", "hello"], "tS'aIni:zl,Et3 tS'aIni:zl,Et3\n");
}

#[test]
fn trace_with_ipa_prints_the_ipa_after_the_trace() {
    speaks(&["-q", "-X", "--ipa", "hello"], "Found: 'hello' [h@loU]  \nhəlˈəʊ\n");
}
