// Tag-ended clauses carry `CLAUSE_INTONATION_NONE`, so their last word gets no tonic.
// Expected values come from C espeak-ng master (f0fb11a8): whole stdout of
// `espeak-ng -q -m -v en --ipa|-x <text>`.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-m", "-v", "en", flag, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

type Case = (&'static str, &'static str, &'static str);

fn check(cases: &[Case]) {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = cases
        .iter()
        .flat_map(|&(text, ipa, x)| [("--ipa", ipa), ("-x", x)].map(move |(f, want)| (text, f, want)))
        .filter_map(|(text, flag, want)| {
            let got = run(flag, text);
            (got != want).then(|| format!("{flag} {text:?}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

#[test]
fn break_ends_a_clause_without_a_tonic() {
    let one_two = ("wˌɒn\ntˈuː\n", "w,0n\nt'u:\n");
    check(&[
        ("one <break/> two", one_two.0, one_two.1),
        ("one <break strength=\"medium\"/> two", one_two.0, one_two.1),
        ("one <break strength=\"strong\"/> two", one_two.0, one_two.1),
        ("one <break time=\"500ms\"/> two", one_two.0, one_two.1),
        ("one <break strength=\"weak\"/> two", "wˈɒn tˈuː\n", "w'0n t'u:\n"),
        ("hello <break/>", "həlˈəʊ\n\n", "h@l'oU\n\n"),
    ]);
}

#[test]
fn prosody_does_not_end_the_clause() {
    check(&[("<prosody pitch=\"high\">up there</prosody>", "ˌʌp ðˈeə\n", ",Vp D'e@\n")]);
}
