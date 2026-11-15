// C puts an extra space between a word and a number that follows it, so a rule such as
// `_) a (_D` (the article "a" before a digit) never matches `a 3`.  Expected values are the
// stdout of C espeak-ng master (f0fb11a8) `espeak-ng -q -v en --ipa|-x <text>`.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-v", "en", flag, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn article_before_a_number_keeps_its_unstressed_form() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases = [
        ("a 3 b", "ɐ θɹˈiː bˈiː\n", "a# Tr'i: b'i:\n"),
        ("a 1", "ɐ wˈɒn\n", "a# w'0n\n"),
        ("a 12 b", "ɐ twˈɛlv bˈiː\n", "a# tw'Elv b'i:\n"),
        ("a 123", "ɐ wˈɒnhˈʌndɹɪdən twˈɛnti θɹˈiː\n", "a# w'0nh'VndrI2d@n tw'Enti Tr'i:\n"),
        ("a <3 b", "ɐ θɹˈiː bˈiː\n", "a#_:_: Tr'i: b'i:\n"),
        ("a (3 b", "ɐ θɹˈiː bˈiː\n", "a#_:_: Tr'i: b'i:\n"),
        ("b 3 c", "bˈiː θɹˈiː sˈiː\n", "b'i: Tr'i: s'i:\n"),
        ("I 3 b", "aɪ θɹˈiː bˈiː\n", "aI Tr'i: b'i:\n"),
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
