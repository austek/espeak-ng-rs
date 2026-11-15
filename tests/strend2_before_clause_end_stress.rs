// A `$strend2` word is promoted only when the words after it stay unstressed once their own
// clause-end stress is applied (`there` is `$u $strend2`).  Expected values are the stdout of
// C espeak-ng master (f0fb11a8) `espeak-ng -q -v en --ipa|-x <text>`.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-v", "en", flag, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn strend2_word_before_a_clause_final_strend2_word_stays_secondary() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases = [
        ("up there", "ˌʌp ðˈeə\n", ",Vp D'e@\n"),
        ("x up there", "ˈɛks ˌʌp ðˈeə\n", "'Eks ,Vp D'e@\n"),
        ("come up there", "kˈʌm ˌʌp ðˈeə\n", "k'Vm ,Vp D'e@\n"),
        ("up there more", "ˌʌp ðeə mˈɔː\n", ",Vp De@ m'o@\n"),
        ("what is up", "wˌɒt ɪz ˈʌp\n", "w,0t Iz 'Vp\n"),
    ];
    let bad: Vec<String> = cases
        .iter()
        .flat_map(|&(t, ipa, x)| [("--ipa", ipa), ("-x", x)].map(move |(f, w)| (t, f, w)))
        .filter_map(|(t, f, want)| {
            let got = run(f, t);
            (got != want).then(|| format!("{f} {t:?}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
