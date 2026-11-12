// `-v` accepts a voice file path relative to the voices directory (`art/eo`, `aav/vi`), as in C.

use std::process::Command;

fn x(voice: &str, text: &str) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .env("ESPEAK_DATA_PATH", env!("CARGO_MANIFEST_DIR"))
        .args(["-q", "-x", "-v", voice, text])
        .output()
        .expect("spawn");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn voice_file_path_art_eo() {
    assert_eq!(x("art/eo", "saluton"), (Some(0), "sal'uton\n".into(), String::new()));
}

#[test]
fn voice_file_path_aav_vi() {
    assert_eq!(x("aav/vi", "xin chao"), (Some(0), "s'i1n_| tS'a:7w_|\n".into(), String::new()));
}
