// Spelled letters and initialisms. Expected output is from C espeak-ng master
// (`--ipa` and `-x`), run for the voice named in each row.

use std::path::Path;
use std::process::Command;

fn run(flag: &str, lang: &str, text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("phontab").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', " "))
}

/// Rows are `(lang, text, ipa, mnemonics)`.
fn check(rows: &[(&str, &str, &str, &str)]) {
    let mut failures = Vec::new();
    for &(lang, text, ipa, mn) in rows {
        let (Some(i), Some(m)) = (run("--ipa", lang, text), run("-x", lang, text)) else {
            eprintln!("[SKIP] no data");
            return;
        };
        if i != ipa {
            failures.push(format!("{lang} {text:?} --ipa: got {i:?}, C says {ipa:?}"));
        }
        if m != mn {
            failures.push(format!("{lang} {text:?} -x: got {m:?}, C says {mn:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn arabic_ain_named_through_another_translator_uses_its_default_stress() {
    check(&[
        ("en", "ع", "ˈaɹəbɪk(ar)ʕʕˈaːjn(en)", "_:'ar@bIk(ar)A-'a:jn(en)"),
        ("de", "ع", "arˈɑbɪʃ(ar)ʕʕˈaːjn(de)", "_:aR'AbIS(ar)A-'a:jn(de)"),
        ("fr", "ع", "(en)ˈaɹəbɪk(fr)ʰcFʀ-ˈaːjn", "_:(en)'ar@bIk(fr)#cFR-'a:jn"),
    ]);
}

#[test]
fn a_word_starting_outside_every_alphabet_is_unpronounceable_and_its_rest_comes_from_the_rules() {
    check(&[
        ("en", "ʃba", "ˌɛʃbˌiːˈeɪ", ",ESb,i:;'eI"),
        ("en", "ʃxa", "ˌɛʃˌɛksˈeɪ", ",ES,Eks'eI"),
        ("en", "ʃhello", "ˈɛʃhˈɛləʊ", "'ESh'EloU"),
    ]);
}

#[test]
fn syllabic_marker_after_a_vowel_of_the_voice_table_is_not_written() {
    check(&[("ku", "ع", "(en)ˈaɹəbɪk(ku)ɔˈaːjn", "_:(en)'ar@bIk(ku)O'a:jn")]);
}
