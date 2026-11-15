// A clause's first word has no separator before it, however many pauses, spaces or markup
// commands precede it in the clause text.
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

#[test]
fn first_word_of_a_clause_follows_its_pauses_directly() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases = [
        ("<speak>&quot;Quoted&quot;</speak>", "kwˈəʊtɪd\n\n", "_:_:kw'oUtI2d\n\n"),
        ("<speak>&lt;tag&gt;</speak>", "tˈaɡ\n\n", "_:_:t'ag\n\n"),
        ("<speak>(7) items.</speak>", "sˈɛvən ˈaɪtəmz\n\n", "_:_:s'Ev@n_:_: 'aIt@mz\n\n"),
        (
            "<speak><prosody rate=\"slow\"><emphasis>slow</emphasis></prosody></speak>",
            "slˈəʊ\n\n",
            "_!sl'oU\n\n",
        ),
        ("<speak><emphasis><emphasis>slow</emphasis></emphasis></speak>", "slˈəʊ\n\n", "_!sl'oU\n\n"),
    ];
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
