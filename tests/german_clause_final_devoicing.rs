// The end of a clause is a word end for word-final devoicing, so a voiced stop
// before a final voiceless one devoices (`sagt` -> `zɑːkt`) even without
// trailing punctuation. Expected strings come from C espeak-ng master (f0fb11a).

use std::path::Path;
use std::process::Command;

fn ipa(text: &str) -> String {
    let bin = env!("CARGO_BIN_EXE_espeak_cli");
    let out = Command::new(bin)
        .args(["-q", "--ipa", "-v", "de", "--", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn check(cases: &[(&str, &str)]) {
    if !Path::new("espeak-ng-data/de_dict").exists() {
        eprintln!("[SKIP] no de data");
        return;
    }
    for (text, want) in cases {
        assert_eq!(&ipa(text), want, "de {text:?}");
    }
}

#[test]
fn voiced_stop_before_final_voiceless_stop_devoices() {
    check(&[
        ("sagt", "zˈɑːkt"),
        ("Sagt", "zˈɑːkt"),
        ("gebt", "ɡˈeːpt"),
        ("du gebt", "duː ɡˈeːpt"),
        ("hat gesagt", "hat ɡəzˈɑːkt"),
    ]);
}

#[test]
fn devoicing_stays_word_final_only() {
    check(&[
        ("sagte", "zˈɑːɡtə"),
        ("sagt ich", "zˈɑːkt ɪç"),
        ("legt", "lˈeːkt"),
        ("tag", "tˈɑːk"),
        ("hund", "hˈʊnt"),
    ]);
}
