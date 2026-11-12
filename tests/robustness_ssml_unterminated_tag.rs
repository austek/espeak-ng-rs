// Fuzz crash: a `<` as the last character sliced the markup text backwards.
// Upstream espeak-ng master (f0fb11a8) exits 0 and prints the words before it:
// `espeak-ng -q -x -m "b <"` -> `b'i:`, `-m "<"` -> empty.

fn cli(text: &str) -> std::process::Output {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-m", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli")
}

#[test]
fn trailing_open_angle_bracket_does_not_panic() {
    for (text, want) in [("b <", "b'i:"), ("ab c<", "'ab s'i:"), ("<", ""), ("<<", "")] {
        let out = cli(text);
        assert!(out.status.success(), "{text:?}: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), want, "{text:?}");
    }
}
