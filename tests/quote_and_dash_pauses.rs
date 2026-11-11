// A closing quote after clause punctuation starts the next clause with its
// pause, and a dash between blanks pauses before the next word.  Expected
// mnemonics are from C espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(lang: &str, text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("en_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|"))
}

fn check(cases: &[(&str, &str, &str)]) {
    for (lang, text, want) in cases {
        let Some(got) = mnemonics(lang, text) else {
            eprintln!("[SKIP] no data");
            return;
        };
        assert_eq!(&got, want, "{lang} {text}");
    }
}

#[test]
fn a_closing_quote_after_a_comma_pauses_at_the_next_clause_start() {
    check(&[("en", "Ez tên,\" wî got.", "'Ez t'e@n|_:_:w'aI g'0t")]);
}

#[test]
fn a_doubled_hyphen_pauses_and_a_pause_already_pending_is_not_repeated() {
    check(&[
        ("ku", "a -- b", "a_:_: b'e"),
        ("ku", "a--b", "a_:_: b'e"),
        ("ku", "a - b -- c", "a_:_: b'e_:_: dZ'e"),
        ("ku", "a - - b", "a_:_: b'e"),
        ("ku", "\"a\" - b", "_:_:a_:_: b'e"),
        ("ku", "a - (b)", "a_:_:_:_: b'e"),
    ]);
}

#[test]
fn a_pause_after_a_number_attaches_to_it() {
    check(&[
        ("ku", "5 - 3", "p'endZ_:_: s'e"),
        ("ku", "3 - a", "s'e_:_: 'a"),
        ("ku", "5 (b)", "p'endZ_:_: b'e"),
        ("ku", "12 ( b", "danzd'E#_:_: b'e"),
    ]);
}

#[test]
fn a_dash_between_blanks_pauses_before_the_next_word() {
    check(&[
        ("en", "one - two three", "w'0n_:_: t'u: Tr'i:"),
        ("en", "one - two - three", "w'0n_:_: t'u:_:_: Tr'i:"),
        ("ku", "me - Ali û Ahmed", "mE#_:_: al'I_:_: u ahm'Ed"),
    ]);
}
