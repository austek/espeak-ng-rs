// `<code>` is an HTML tag that writes no space (C `HTML_NOSPACE`), like `<b>` and `<em>`.
// Expected values are the whole stdout of C espeak-ng master (f0fb11a8)
// `espeak-ng -q -m -v en --ipa|-x <text>`.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-m", "-v", "en", flag, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn code_tag_joins_the_text_around_it() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases = [
        ("<code>x</code>y", "ˌɛkswˈaɪ\n", ",Eksw'aI\n"),
        ("<code>x</code> y", "ˈɛks wˈaɪ\n", "'Eks w'aI\n"),
        ("<b>x</b>y", "ˌɛkswˈaɪ\n", ",Eksw'aI\n"),
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
