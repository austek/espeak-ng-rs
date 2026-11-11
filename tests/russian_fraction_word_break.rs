// A Russian fraction numerator of 100 or more follows the decimal word without
// a word break (C's LookupNum3 opens with a break only below 100). Expected
// mnemonics are from C espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("ru_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", "ru", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', " "))
}

#[test]
fn hundreds_numerator_joins_the_decimal_word() {
    let cases = [
        ("1,234", "V#d;'in_ _:'i_:dv;'es;t;I t@-r;'i_:tsVt;tS;It'yR;I_t'ys;itS;nYx"),
        ("12,100", "dv;In'A_:tsVt; _:'i_:st'o _t'ys;itS;nYx"),
        ("1,101", "V#d;'in_ _:'i_:st'o V#dn'A_t'ys;itS;nV#ja"),
        ("1,5", "V#d;'in_ _:'i_: p;'At;_d;Is;'AtYx"),
    ];
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no ru data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
