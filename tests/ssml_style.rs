// `<tts:style>` punctuation and capitals, against C espeak-ng master (f0fb11a8).
// Expected values: whole stdout of `espeak-ng -q -m -v en --ipa|-x <text>`.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-m", "-v", "en", flag, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn have_data() -> bool {
    let ok = std::path::Path::new("espeak-ng-data/en_dict").exists();
    if !ok {
        eprintln!("[SKIP] no data");
    }
    ok
}

/// `(input, --ipa, -x)` as printed by C.
type Case = (&'static str, &'static str, &'static str);

fn check(cases: &[Case]) {
    if !have_data() {
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
fn announced_brackets_and_quotes_pause_before_the_next_word() {
    check(&[
        (
            r#"<tts:style field="punctuation" mode="all">ok (yes) "q"</tts:style>"#,
            "ˌəʊkˈeɪ lˈɛftpɐɹˌɛn jˈɛs ɹˈaɪtpɐɹˌɛn kwˈəʊts kjˈuː kwˈəʊts\n",
            ",oUk'eI l'Eftpa#r,En_: j'Es r'aItpa#r,En_: kw'oUts_: kj'u: kw'oUts\n",
        ),
        (
            r#"<tts:style field="punctuation" mode="all">x [y] {z}</tts:style>"#,
            "ˈɛks lˈɛftbɹakɪt wˈaɪ ɹˈaɪtbɹakɪt lˈɛftbɹeɪs zˈɛd ɹˈaɪtbɹeɪs\n",
            "'Eks l'EftbrakI2t_: w'aI r'aItbrakI2t_: l'EftbreIs_: z'Ed r'aItbreIs\n",
        ),
    ]);
}
