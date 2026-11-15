// tests/ssml_emphasis_multiword_entries.rs
//
// A multi-word list entry (`$2` skipwords) is not used when any word it covers is
// emphasized (C `LookupDict2`), while a `<mark/>` between the words leaves it intact.
// Expected values are what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

const CASES: [(&str, &str, &str); 9] = [
    ("<emphasis>bon voyage</emphasis>", "bˈɒn vˈɔɪɪdʒ", "_!b'0n_! v'OII2dZ"),
    ("<emphasis>each of</emphasis>", "ˈiːtʃ ˈɒv", "_!'i:tS_! '0v"),
    ("<emphasis>each</emphasis> of", "ˈiːtʃ ɒv", "_!'i:tS 0v"),
    ("each <emphasis>of</emphasis>", "ˌiːtʃ ˈɒv", ",i:tS_! '0v"),
    ("bon <mark name=\"a\"/>voyage", "bˌɒn vɔɪˈɑːʒ", "b,0n vOI'A:Z"),
    ("each <mark name=\"a\"/>of", "ˈiːtʃ əv", "'i:tS @v"),
    ("bon voyage", "bˌɒn vɔɪˈɑːʒ", "b,0n vOI'A:Z"),
    ("each of", "ˈiːtʃ əv", "'i:tS @v"),
    ("e.g. case", "fˌɔːɹɛɡzˈampəl kˈeɪs", "f,O@rEgz'aamp@L k'eIs"),
];

// C also ends these with a pause (`_!`) that the port does not print yet.
const DOTTED_IPA_ONLY: [(&str, &str); 3] = [
    ("the <emphasis>e.g.</emphasis> case", "ðə ˌiːdʒˈiː kˈeɪs"),
    ("<emphasis>a.k.a.</emphasis>", "ˌeɪkˌeɪˈeɪ"),
    ("<emphasis level=\"strong\">a.k.a.</emphasis>", "ˌeɪkˌeɪˈeɪ"),
];

#[test]
fn emphasized_words_block_multiword_entries() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let full = CASES.iter().flat_map(|&(t, i, x)| [("--ipa", i, t), ("-x", x, t)]);
    let ipa_only = DOTTED_IPA_ONLY.iter().map(|&(t, i)| ("--ipa", i, t));
    let bad: Vec<String> = full
        .chain(ipa_only)
        .filter_map(|(f, want, t)| {
            let got = run("en", f, &format!("<speak>{t}</speak>"));
            (got != want).then(|| format!("{f} {t}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
