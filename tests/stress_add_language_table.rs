// A variant's `stressAdd` lines are added to the language's own stress lengths
// (de, fr, es each have their own table), not to the default English table.
// Expected WAV sizes in bytes come from the C binary (`--stdout`).

use std::process::Command;

fn wav_len(voice: &str, text: &str) -> usize {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .env("ESPEAK_DATA_PATH", env!("CARGO_MANIFEST_DIR"))
        .args(["--stdout", "-v", voice, text])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    out.stdout.len()
}

#[test]
fn variant_stress_add_uses_the_language_table() {
    let cases = [
        ("de+f2", "Das ist ein Test heute", 66816),
        ("de+m3", "Das ist ein Test heute", 65938),
        ("fr+f2", "Bonjour tout le monde", 50048),
        ("fr+m4", "Bonjour tout le monde", 53202),
        ("es+f2", "Hola amigos buenos dias", 74128),
    ];
    for (voice, text, want) in cases {
        assert_eq!(wav_len(voice, text), want, "-v {voice}");
    }
}
