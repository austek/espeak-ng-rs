// tests/ssml_say_as_digit_groups.rs
//
// C breaks a run of 5 to 32 digits into the words `break_numbers` names before any
// number is read (translate.c:1714), so `tts:digits` and `telephone` speak each group
// digit by digit as a word of its own.  Expected values are what C espeak-ng master
// (f0fb11a8) prints.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn say_as(mode: &str, digits: &str) -> String {
    format!("<speak><say-as interpret-as=\"{mode}\">{digits}</say-as></speak>")
}

const CASES: [(&str, &str, &str, &str); 7] = [
    ("tts:digits", "1234567", "wˈɒn tˈuːθɹˈiːfˈɔː fˈaɪvsˈɪkssˈɛvən", "w'0n t'u:Tr'i:_!f'o@ f'aIvs'Iks_!s'Ev@n"),
    ("tts:digits", "90210", "nˈaɪnzˈiəɹəʊ tˈuːwˈɒnzˈiəɹəʊ", "n'aInz'i@roU_! t'u:w'0n_!z'i@roU"),
    ("tts:digits", "12345", "wˈɒntˈuː θɹˈiːfˈɔːfˈaɪv", "w'0nt'u:_! Tr'i:f'o@_!f'aIv"),
    ("tts:digits", "123456", "wˈɒntˈuːθɹˈiː fˈɔːfˈaɪvsˈɪks", "w'0nt'u:_!Tr'i: f'o@f'aIv_!s'Iks"),
    ("telephone", "5551234", "fˈaɪv fˈaɪvfˈaɪvwˈɒn tˈuːθɹˈiːfˈɔː", "f'aIv f'aIvf'aIv_!w'0n t'u:Tr'i:_!f'o@"),
    (
        "telephone",
        "0123456789",
        "zˈiəɹəʊ wˈɒntˈuːθɹˈiː fˈɔːfˈaɪvsˈɪks sˈɛvənˈeɪtnˈaɪn",
        "z'i@roU w'0nt'u:_!Tr'i: f'o@f'aIv_!s'Iks s'Ev@n'eIt_!n'aIn",
    ),
    ("tts:digits", "1234", "wˈɒntˈuːθɹˈiːfˈɔː", "w'0nt'u:_!Tr'i:f'o@_!"),
];

#[test]
fn long_digit_runs_break_into_groups() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .flat_map(|&(m, d, i, x)| [("--ipa", i, m, d), ("-x", x, m, d)])
        .filter_map(|(f, want, m, d)| {
            let got = run(f, &say_as(m, d));
            (got != want).then(|| format!("{f} {m} {d}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
