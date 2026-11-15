// tests/dot_clause_end.rs
//
// Whether a '.' set apart by blanks stays in the clause is decided by what follows it, as in
// C's ReadClause (readclause.c:817-922).  Expected values are what C espeak-ng master
// (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang])
        .arg(text)
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: &[Case] = &[
    ("en", "He said .. Then left", "hiː sˈɛd\nðˈɛn lˈɛft", "hi: s'Ed\nD'En l'Eft"),
    ("en", "He said .. 5 left", "hiː sˈɛd\nfˈaɪv lˈɛft", "hi: s'Ed\nf'aIv l'Eft"),
    ("en", "He said ..", "hiː sˈɛd", "hi: s'Ed"),
    ("ar", "قال .. ثم غادر", "qˈaːl\nθˈumma ɣˈaːdar", "q'a:l\nT'umma gH'a:daR"),
    ("hi", "उसने कहा .. फिर गया", "ˈʊsnˌeː kˈʌhaː\npʰˈɪɾ ɡˈʌjaː", "'Usn,e: k'VHa:\np#'Ir g'Vja:"),
    ("fa", "گفت .. سپس رفت", "ɡˈoft\nsˈepas rˈaft", "g'oft\ns'epas R'aft"),
];

#[test]
fn a_spaced_dot_before_a_clause_ending_dot_is_not_read() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .flat_map(|&(l, t, i, x)| [(l, "--ipa", i, t), (l, "-x", x, t)])
        .filter_map(|(l, f, want, t)| {
            let got = run(l, f, t);
            (got != want).then(|| format!("{l} {f} {t:?}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
