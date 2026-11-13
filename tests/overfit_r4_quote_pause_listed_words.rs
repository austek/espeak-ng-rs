// A quote in a word that the dictionary lists, or that a suffix rule covers,
// raises no pause before the next word.  Values from upstream espeak-ng master
// (f0fb11a8), `espeak-ng -q -x -v <lang>`.

fn cli(lang: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn a_covered_quote_adds_no_pause() {
    let cases = [
        ("nl", "z'n auto staat daar", "z@n _!'VUto: st'a:! t'a:r"),
        ("nl", "Anna's fiets", "_!'Ana:s f'its"),
        ("nl", "auto's rijden snel", "_!'VUto:s r'EId@n sn'El"),
        ("af", "ma's kar is rooi", "m'A:s k'ar _|Is r'oI"),
        ("fr", "quelqu'un est venu", "kElk,W~ E v@n'y"),
    ];
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|&(lang, text, want)| {
            let got = cli(lang, text);
            (got != want).then(|| format!("{lang} {text:?}\n  got  {got}\n  want {want}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
