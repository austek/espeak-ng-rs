// C splits the integer part of a decimal at its thousands separator (or after
// five digits) into words of their own: the thousands word ends in a no-link
// pause and the last group carries the point and the fraction. Expected
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
fn separated_four_digit_integer_splits_before_the_fraction() {
    check(&[
        (
            "en",
            "1,234.56",
            "w'0n T'aUz@nd_! t'u:h'VndrI2d@n T'3:ti f'o@ pOInt f'aIv s'Iks",
        ),
        (
            "es",
            "1.234,56",
            "m'il_! dosTj'entos t**,eIntaikw'at**o k,oma Tinkw,Entais'eIs",
        ),
        ("fr", "1 234,56", "m'il_! dYsA~ trA~tkatr virgyl sE~kA~ts'is"),
        ("en", "1,234.5.6", "w'0n T'aUz@nd_! t'u:h'VndrI2d@n T'3:ti f'o@ pOInt f'aIvpOInt s'Iks"),
    ]);
}

#[test]
fn long_integer_splits_before_the_fraction() {
    check(&[
        ("fr", "12345,6", "duz m'il_! trwasA~ karA~tsE~k virgyl s'is"),
        ("hu", "2 000,5", "k'e:tEzER2 _! 'Ege:s Yt2tizEd"),
    ]);
}
