// tests/ssml_say_as_unrecognised_modes.rs
//
// C's `mnem_interpret_as` knows only characters, tts:char, tts:key, tts:digits
// and telephone.  Every other `interpret-as` value (ordinal, time, digits,
// spell-out, ...) reads its content as ordinary text.  Expected values are what
// C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn unrecognised_interpret_as_reads_plain_text() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases = [
        ("<speak><say-as interpret-as=\"ordinal\">3</say-as></speak>", "θɹˈiː", "Tr'i:"),
        (
            "<speak><say-as interpret-as=\"ordinal\">21</say-as> floor</speak>",
            "twˈɛnti wˈɒn flˈɔː",
            "tw'Enti w'0n fl'o@",
        ),
        (
            "<speak><say-as interpret-as=\"digits\">1234</say-as></speak>",
            "wˈɒn θˈaʊzənd tˈuːhˈʌndɹɪdən θˈɜːti fˈɔː",
            "w'0n T'aUz@nd t'u:h'VndrI2d@n T'3:ti f'o@",
        ),
        ("<speak><say-as interpret-as=\"spell-out\">hi</say-as></speak>", "hˈaɪ", "h'aI"),
        (
            "<speak><say-as interpret-as=\"time\">14:30</say-as></speak>",
            "fˈɔːtiːn θˈɜːti",
            "f'o@ti:n T'3:ti",
        ),
    ];
    let bad: Vec<String> = cases
        .iter()
        .flat_map(|&(t, i, x)| [("--ipa", i, t), ("-x", x, t)])
        .filter_map(|(f, want, t)| {
            let got = run(f, t);
            (got != want).then(|| format!("{f} {t:?}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
