// tests/standalone_dot_in_clause.rs
//
// A '.' that does not touch a letter on its left stays in the clause text and is read by
// name (translate.c:1509-1520 drops a dot only after an alphabetic character that is
// followed by a space).  That holds for a dot after an embedded SSML command, for a dot
// that follows a dotted abbreviation across a command, and for a spaced dot in plain
// text.  Expected values are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let markup: &[&str] = if text.contains('<') { &["-m"] } else { &[] };
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang])
        .args(markup)
        .arg(text)
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 17] = [
    ("en", "<speak>C<mark name=\"x\"/>. and D</speak>", "sˈiː dˈɒt and dˈiː", "s'i: d'0t_:_: and d'i:"),
    ("en", "3.5 . and 4.5", "θɹˈiː pɔɪnt fˈaɪv dˈɒt and fˈɔː pɔɪnt fˈaɪv","Tr'i: pOInt f'aIv d'0t_:_: and f'o@ pOInt f'aIv"),
    ("en", "<speak>N.A.S.A<mark name=\"m\"/>. is here</speak>", "ˌɛnˌeɪˌɛsˈeɪ  ɪz hˈiə",",En,eI,Es'eI _| Iz h'i@3"),
    ("fr", "<speak>Bonjour<mark name=\"m\"/>. monde</speak>", "bɔ̃ʒˈuʁ  mˈɔ̃d","bO~Z'ur _| m'O~d"),
    ("fr", "<speak><emphasis>A</emphasis>. et B</speak>", "ˈa  e bˈe","_!'a _| e b'e"),
    ("it", "<speak>A<mark name=\"m\"/>. poi</speak>", "ˈaː  pˈɔi","'a: _| p'Oi"),
    ("ru", "<speak>А<mark name=\"m\"/>. потом</speak>", "ˈɑ  pˈotʌm","'A _| p'otVm"),
    ("en", "He said . then left.", "hiː sˈɛd dˈɒt ðˈɛn lˈɛft", "hi: s'Ed d'0t D'En l'Eft"),
    (
        "en",
        "Version 2.5 . is out",
        "vˈɜːʃən tˈuː pɔɪnt fˈaɪv dˈɒt ɪz ˈaʊt",
        "v'3:S@n t'u: pOInt f'aIv d'0t Iz 'aUt",
    ),
    ("en", "He said .. then left", "hiː sˈɛd dˈɒt ðˈɛn lˈɛft", "hi: s'Ed d'0t D'En l'Eft"),
    ("es", "Dijo . luego se fue.", "dˈixo pˈunto luˈeɣo se fwˈe", "d'ixo p'unto lu'eQo se fw'e"),
    ("en", "cat <emphasis>.</emphasis> next.", "kˌat dˈɒt nˈɛkst", "k,at_! d'0t n'Ekst"),
    ("en", "<speak>U.S.A<emphasis>.</emphasis> next.</speak>", "jˌuːˌɛsˈeɪ nˈɛkst", "j,u:,Es'eI_! n'Ekst"),
    ("en", "<speak><emphasis>A</emphasis>.</speak>", "ˈeɪ dˈɒt", "_!'eI d'0t"),
    ("en", "<speak>A<mark name=\"m\"/>.</speak>", "ˈeɪ dˈɒt", "'eI d'0t"),
    ("en", "<speak><prosody rate=\"slow\">A</prosody>.</speak>", "ˈeɪ dˈɒt", "'eI d'0t"),
    ("en", "<speak>U.S.<emphasis>A</emphasis>.</speak>", "jˌuːˈɛs\nˈeɪ dˈɒt", "j,u:'Es\n'eI d'0t"),
];

#[test]
fn a_dot_without_a_letter_before_it_is_read_as_c_reads_it() {
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
