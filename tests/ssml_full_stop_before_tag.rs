// tests/ssml_full_stop_before_tag.rs
//
// C's `end_clause_after_tag`: a full stop followed (past blanks) by an SSML tag
// does not end its clause, the tag's terminator does, so no empty clause line
// appears.  Expected values are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn full_stop_before_tag_adds_no_empty_clause() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases = [
        (
            "<speak><s>Hello there.</s><s>How are you</s></speak>",
            "həlˈəʊ ðˈeə\n\nhˈaʊ ɑː juː",
            "h@l'oU D'e@\n\nh'aU A@ ju:",
        ),
        (
            "<speak><p>First paragraph.</p><p>Second one here</p></speak>",
            "fˈɜːst pˈaɹəɡɹˌaf\n\nsˈɛkənd wˈɒn hˈiə",
            "f'3:st p'ar@gr,aaf\n\ns'Ek@nd w'0n h'i@3",
        ),
        (
            "<speak>Stop. <break strength=\"strong\"/> go</speak>",
            "stˈɒp\nɡˈəʊ",
            "st'0p\ng'oU",
        ),
        ("<speak><s>One.</s> <s>Two.</s></speak>", "wˈɒn\n\ntˈuː", "w'0n\n\nt'u:"),
    ];
    let bad: Vec<String> = cases
        .iter()
        .flat_map(|&(t, i, x)| [("--ipa", i, t), ("-x", x, t)])
        .filter_map(|(f, want, t)| {
            let got = run(f, t);
            (got != want).then(|| format!("{f} {t:?}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
