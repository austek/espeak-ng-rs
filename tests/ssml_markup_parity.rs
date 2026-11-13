// tests/ssml_markup_parity.rs
//
// `-m` phoneme output against C espeak-ng master (f0fb11a8).  Expected values
// are what the C binary prints for `--ipa` and `-x`.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// `(input, --ipa, -x)` as printed by C.
type Case = (&'static str, &'static str, &'static str);

fn check(cases: &[Case]) {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
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

/// `ignore_text` is one flag: any closing `sub`/`metadata`/`script`/`style` clears it.
#[test]
fn ignored_text_elements_and_sub_flag() {
    check(&[
        ("<metadata>foo</metadata> bar", "bˈɑː", "b'A@"),
        ("a <script> c </script> d", "ɐ dˈiː", "a# d'i:"),
        (
            "<sub alias=\"A B\">xx <metadata>q</metadata> zz</sub> y",
            "ɐ bˈiː zˌɛdzˈɛd wˈaɪ",
            "a# b'i: z,Edz'Ed w'aI",
        ),
    ]);
}

/// C starts a tag only when `<` is followed by `/`, a letter, `!` or `?`.
#[test]
fn lt_before_a_non_tag_character_is_literal() {
    check(&[
        ("a < b", "ɐ bˈiː", "a#_:_: b'i:"),
        ("1 < 2", "wˈɒn tˈuː", "w'0n_:_: t'u:"),
        ("3 < 4 and 5 > 2", "θɹˈiː fˈɔː and fˈaɪv tˈuː", "Tr'i:_:_: f'o@_:_: and f'aIv_:_: t'u:"),
    ]);
}
