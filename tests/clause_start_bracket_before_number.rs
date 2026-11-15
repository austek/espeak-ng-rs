// A bracket that opens the clause pauses, and a number right after it starts without a word
// break.  Expected values are the stdout of C espeak-ng master (f0fb11a8)
// `espeak-ng -q -v en --ipa|-x <text>`.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-v", "en", flag, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn number_after_an_opening_bracket_has_no_leading_word_break() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases = [
        ("(1)", "wˈɒn\n", "_:_:w'0n\n"),
        ("(12)", "twˈɛlv\n", "_:_:tw'Elv\n"),
        ("[1]", "wˈɒn\n", "_:_:w'0n\n"),
        ("<3", "θɹˈiː\n", "_:_:Tr'i:\n"),
        ("(1) a", "wˈɒn ˈeɪ\n", "_:_:w'0n_:_: 'eI\n"),
        ("\"3\" times", "θɹˈiː tˈaɪmz\n", "_:_:Tr'i:_:_: t'aImz\n"),
        ("(1.5)", "wˈɒn pɔɪnt fˈaɪv\n", "_:_:w'0n pOInt f'aIv\n"),
        ("(-3)", "θɹˈiː\n", "_:_:Tr'i:\n"),
        ("x (1)", "ˈɛks wˈɒn\n", "'Eks_:_: w'0n\n"),
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
