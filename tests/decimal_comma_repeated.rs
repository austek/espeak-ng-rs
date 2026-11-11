// In a decimal-comma language C speaks no second separator in `1,234,567`: the
// digits after it are a cardinal of their own. Expected mnemonics are from C
// espeak-ng master.

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
fn second_comma_is_not_spoken() {
    check(&[
        ("es", "1,234,567", "'uno k,omaDosTj'entos t**,eIntaikw'at**o kinj'Entos sEs,Entaisj'ete"),
        ("de", "1,23,45", "_'aIns _kOmA: tsv'aI dr'aI f'ynf _|Untf'IrtsIC"),
        ("nl", "1,234,567", "_! 'e:n _k,Oma: tv#,e: dr,i v,ir v'EIfh,Ond@rt z,e:v@nEnz,Est@x"),
    ]);
}
