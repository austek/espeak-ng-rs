// English words spoken inside a German or Dutch voice keep their English
// voicing: word-final devoicing applies only to the voice's own phonemes.
// Expected strings come from C espeak-ng master (f0fb11a).

use std::path::Path;
use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let bin = env!("CARGO_BIN_EXE_espeak_cli");
    let out = Command::new(bin)
        .args(["-q", flag, "-v", lang, "--", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn check(lang: &str, flag: &str, cases: &[(&str, &str)]) {
    if !Path::new(&format!("espeak-ng-data/{lang}_dict")).exists() {
        eprintln!("[SKIP] no {lang} data");
        return;
    }
    for (text, want) in cases {
        assert_eq!(&run(lang, flag, text), want, "{lang} {text:?}");
    }
}

#[test]
fn german_english_loanwords_keep_final_voiced_obstruents() {
    check(
        "de",
        "--ipa",
        &[
            ("job", "(en)dʒˈɒb(de)"),
            ("size", "(en)sˈaɪz(de)"),
            ("save", "(en)sˈeɪv(de)"),
            ("update", "(en)ˈʌpdeɪt(de)"),
            ("word", "(en)wˈɜːd(de)"),
            ("change", "(en)tʃˈeɪndʒ(de)"),
            ("snob", "(en)snˈɒb(de)"),
            ("wir sagen job", "viːɾ zˈɑːɡən (en)dʒˈɒb(de)"),
        ],
    );
}

#[test]
fn german_words_still_devoice_before_and_after_a_loanword() {
    check(
        "de",
        "--ipa",
        &[("tag", "tˈɑːk"), ("hund", "hˈʊnt"), ("der tag", "dɛɾ tˈɑːk")],
    );
}

#[test]
fn dutch_english_loanwords_keep_final_voiced_obstruents() {
    check("nl", "--ipa", &[("website", "(en)wˈɛbsaɪt(nl)")]);
    check("nl", "-x", &[("website", "(en)w'EbsaIt(nl)")]);
}
