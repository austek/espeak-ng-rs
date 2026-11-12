// tests/sep_lengthen.rs
//
// With `--sep`, the length mark belongs to the phoneme it follows: C writes
// `H_'a:_l_i_k`, never `H_'a_:_l_i_k`.  Expectations taken from upstream
// `espeak-ng` (f0fb11a8) `-q -x --sep=_`.

use std::path::Path;
use std::process::Command;

fn run(args: &[&str]) -> String {
    let bin = env!("CARGO_BIN_EXE_espeak_cli");
    let out = Command::new(bin).args(args).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn arabic_long_vowels_keep_the_mark_attached() {
    if !Path::new("espeak-ng-data/ar_dict").exists() {
        eprintln!("[SKIP] no ar data");
        return;
    }
    assert_eq!(
        run(&["-v", "ar", "-q", "-x", "--sep=_", "كيف حالك"]),
        "k_'a_j_f_a H_'a:_l_i_k"
    );
    assert_eq!(
        run(&["-v", "ar", "-q", "-x", "--sep=_", "شكرا جزيلا"]),
        "S_k_R_'a: dZ_z_'i:_l_a:"
    );
}

#[test]
fn english_long_vowels_keep_the_mark_attached() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no en data");
        return;
    }
    assert_eq!(
        run(&["-q", "-x", "--sep=_", "hello world: see"]),
        "h_@_l_'oU w_'3:_l_d\ns_'i:"
    );
}
