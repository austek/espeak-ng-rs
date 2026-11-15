// tests/ssml_dot_next_to_tag.rs
//
// A full stop that touches an SSML tag stays in the clause text (readclause.c:817), so
// translate.c reads it like any other character: spoken by name after a tag, dropped
// after a word, part of an ordinal after a number, part of a dictionary phrase after
// `e.g`.  Expected values are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 8] = [
    ("en", "<speak><emphasis>x</emphasis>.</speak>", "ˈɛks dˈɒt", "_!'Eks d'0t"),
    ("en", "<speak><prosody rate=\"slow\">x</prosody>.</speak>", "ˈɛks dˈɒt", "'Eks d'0t"),
    ("en", "<speak>x<mark name=\"m\"/>.</speak>", "ˈɛks dˈɒt", "'Eks d'0t"),
    ("nl", "<speak><emphasis>iedereen</emphasis>.</speak>", "ˌidɪːrˈeːn pˈɵnt", "_!_|,idI:r'e:n p'8nt"),
    (
        "de",
        "<speak>Ich habe 123.</speak>",
        "ɪç hɑːbə ˈaɪnhˈʊndɜt dɾˈaɪ ʊnttsvˈantsɪçstə",
        "_|IC hA:b@ _'aInh'Und3t dr'aI _|Unttsv'antsICst@",
    ),
    ("en", "<speak>e.g. <break/>this one.</speak>", "fˌɔːɹɛɡzˈampəl\nðˈɪswˌɒn", "f,O@rEgz'aamp@L\nD'Isw,0n"),
    (
        "en",
        "<speak>e.g. <emphasis>this</emphasis> one.</speak>",
        "fˌɔːɹɛɡzˌampəl ðˈɪs wˌɒn",
        "f,O@rEgz,aamp@L_! D'Is w,0n",
    ),
    ("en", "<speak>Wait.<break/>now</speak>", "wˈeɪt\nnˈaʊ", "w'eIt\nn'aU"),
];

#[test]
fn dot_next_to_a_tag_is_read_as_c_reads_it() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .flat_map(|&(l, t, i, x)| [(l, "--ipa", i, t), (l, "-x", x, t)])
        .filter_map(|(l, f, want, t)| {
            let got = run(l, f, t);
            (got != want).then(|| format!("{l} {f} {t:?}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
