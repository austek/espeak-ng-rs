// tests/ssml_say_as_key_digits_and_symbols.rs
//
// `tts:key` (C `SAYAS_KEY`): a run of digits longer than one is looked up as `_<digits>` and
// otherwise read digit by digit, and a one-character key such as `+` that the list replaces
// by a word reads that word under its `_`-prefixed key name (`pl'u:z`).  Expected values are
// what C espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

const CASES: [(&str, &str, &str, &str); 12] = [
    ("en", "123", "wˈɒntˈuːθɹˈiː", "w'0nt'u:_!Tr'i:"),
    ("en", "abc123", "ˈabk wˈɒntˈuːθɹˈiː", "'abk w'0nt'u:_!Tr'i:"),
    ("en", "F123", "ˈɛf wˈɒntˈuːθɹˈiː", "'Ef_! w'0nt'u:_!Tr'i:"),
    ("en", "20", "tˈuːzˈiəɹəʊ", "t'u:z'i@roU_!"),
    ("en", "007", "zˈiəɹəʊzˈiəɹəʊsˈɛvən", "z'i@roUz'i@roU_!s'Ev@n"),
    ("en", "12", "twˈɛlv", "tw'Elv"),
    ("de", "45", "fˈiːɾfˈʏnf", "f'i:rf'ynf_!"),
    ("es", "456", "kwˈatɾoθˈinkosˈeɪs", "kw'at**oT'inko_!s'eIs"),
    ("en", "+", "plˈuːz", "pl'u:z"),
    ("en", "ctrl+c", "ktrl plˈuːz sˈiː", "ktrl pl'u:z s'i:_!"),
    ("en", "alt+f4", "ˈɒlt plˈuːz ˈɛf fˈɔː", "'0lt pl'u:z 'Ef_! f'o@_!"),
    ("de", "ctrl+c", "ktɾl plˈʊs tsˈeː", "ktrl pl'Us ts'e:_!"),
];

#[test]
fn key_digits_and_symbols_read_like_c() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .flat_map(|&(l, k, i, x)| [(l, "--ipa", i, k), (l, "-x", x, k)])
        .filter_map(|(l, f, want, k)| {
            let text = format!("<speak><say-as interpret-as=\"tts:key\">{k}</say-as></speak>");
            let got = run(l, f, &text);
            (got != want).then(|| format!("{l} {f} {k}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
