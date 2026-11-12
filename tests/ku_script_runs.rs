// Arabic-script words read by a Kurdish (Latin) voice: spelled letters and
// words handed to `ar` run together without a doubled word break or a stray
// switch back.  Values from upstream espeak-ng master (f0fb11a8),
// `espeak-ng -q -x -v ku`.

fn cli(text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", "ku", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim_end().to_string()
}

#[test]
fn spelled_and_switched_words_join_as_c_does() {
    let cases = [
        (
            "دوو هە دوو",
            "(ar)d'u:w_: (en)'ar@bIk(ku)h'a:?_: (en)'ar@bIk(ku)tipa_|SES_|dE_|p'endZ_:(ar) d'u:w(ku)",
        ),
        (
            "هە هە",
            "_: (en)'ar@bIk(ku)h'a:?_: (en)'ar@bIk(ku)tipa_|SES_|dE_|p'endZ_:_: (en)'ar@bIk(ku)h'a:?_: (en)'ar@bIk(ku)tipa_|SES_|dE_|p'endZ_:",
        ),
        ("ئ ئ", "(ar)?_: (en)'ar@bIk(ku)h'amza O'ala: ?alj'a:?"),
        (
            "دوو و سێ",
            "(ar)d'u:w w'a_: (en)'ar@bIk(ku)s'i:n_: (en)'ar@bIk(ku)tipa_|SES_|dZ'e_|E_:",
        ),
        ("ئ دوو", "(ar)? d'u:w(ku)"),
        ("دوو و", "(ar)d'u:w_: (en)'ar@bIk(ku)w'a:w"),
    ];
    let mut failures = Vec::new();
    for (text, want) in cases {
        let got = cli(text);
        if got != want {
            failures.push(format!("{text:?}\n  got  {got}\n  want {want}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
