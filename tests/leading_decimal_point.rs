// A decimal separator that opens a word (`.5`, `,5`) is not a number to C: the
// rules speak the separator, then every digit is read on its own. Expected
// mnemonics are from C espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(lang: &str, text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join(format!("{lang}_dict")).exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', " "))
}

fn check(cases: &[(&str, &str, &str)]) {
    for (lang, text, want) in cases {
        let Some(got) = mnemonics(lang, text) else {
            eprintln!("[SKIP] no {lang} data");
            return;
        };
        assert_eq!(got, *want, "{lang} {text}");
    }
}

#[test]
fn dot_is_glued_to_the_digits() {
    check(&[
        ("en", ".5", "d0tf'aIv"),
        ("en", "No.5", "n'oU d0tf'aIv"),
        ("en", "x.5", "'Eks d0tf'aIv"),
        ("en", ".25", "d0tt'u:f'aIv_!"),
        ("en", ".05", "d0tz'i@roUf'aIv_!"),
        ("en", ".123456", "d0tw'0nt'u:_!Tr'i:f'o@_!f'aIvs'Iks_!"),
    ]);
}

#[test]
fn languages_whose_rules_drop_the_dot_read_only_digits() {
    check(&[("ku", ".5", "p'endZ"), ("ku", "x.5", "x'e p'endZ"), ("ar", ".5", "x'amsa")]);
}

#[test]
fn comma_languages_speak_the_comma() {
    check(&[("de", ",5", "k,Omaf'ynf")]);
}

#[test]
fn a_unit_after_a_bare_decimal_is_not_expanded() {
    check(&[("en", ".5 kg", "d0tf'aIv k,eIdZ'i:")]);
}
