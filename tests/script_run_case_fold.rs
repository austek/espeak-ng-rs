// C's `TranslateClause` compares the current letter as written with the
// previous letter already lower-cased: U+2126 (ohm) before a native word is a
// Greek omega there, U+212A (kelvin) a Latin k, and an upper-case Cyrillic
// letter stays below the native-script offset for the hyphen split.
// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <lang>`.

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
fn a_letter_that_lower_cases_into_another_script_starts_a_new_word() {
    let rows = [
        ("ru", "\u{2126}дом", "(el)om'eQa(ru) d'om"),
        ("ru", "\u{212a}дом", "(en)k'eI(ru)d'om"),
        ("ru", "\u{1e9e}дом", "(en)S,A@p'Es(ru)d'om"),
        ("en", "\u{2126}house", "'oUmIg@_ h'aUs"),
        ("am", "\u{2126}ቤት", "(el)om'eQa(am) b'et"),
        ("sr", "\u{2126}кућа", "'omeg& k'utS;&"),
    ];
    for (lang, text, want) in rows {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}

#[test]
fn an_upper_case_native_letter_does_not_split_a_latin_run() {
    let rows = [("kk", "ЖУРНАЛjournal", "Zw*n'A L@(en)dZ'3:n@L(kk)")];
    for (lang, text, want) in rows {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}
