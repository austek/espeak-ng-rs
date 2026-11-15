// An emphasized word's tonic stress mark is not a phoneme: neighbouring phoneme programs
// (`nextPh`, `prevPh`) must see through it, and earlier stresses stay as they were.
// Expected values come from C espeak-ng master (f0fb11a8): whole stdout of
// `espeak-ng -q -m -v <voice> --ipa|-x <text>`.

use std::process::Command;

fn run(voice: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-m", "-v", voice, flag, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

fn check(cases: &[Case]) {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = cases
        .iter()
        .flat_map(|&(voice, text, ipa, x)| {
            [("--ipa", ipa), ("-x", x)].map(move |(f, want)| (voice, text, f, want))
        })
        .filter_map(|(voice, text, flag, want)| {
            let got = run(voice, flag, text);
            (got != want).then(|| format!("{voice} {flag} {text:?}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

#[test]
fn emphasized_word_phonemes_see_through_the_tonic_mark() {
    check(&[
        ("en", "<speak><emphasis>red</emphasis></speak>", "ɹˈɛd\n\n", "_!r'Ed\n\n"),
        ("en", "<speak><emphasis>really</emphasis></speak>", "ɹˈiəlɪ\n\n", "_!r'i@lI\n\n"),
        ("en", "<speak><emphasis>pm</emphasis></speak>", "pˌiːˈɛm\n\n", "_!p,i:;'Em\n\n"),
        ("it", "<speak><emphasis>nazione</emphasis></speak>", "natsiˈone\n\n", "_!natsi;'one\n\n"),
    ]);
}
