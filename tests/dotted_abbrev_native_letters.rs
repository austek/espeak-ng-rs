// tests/dotted_abbrev_native_letters.rs
//
// A dotted run is handed to another voice only when the voice cannot name its letters: mk reads
// Latin letters through its own rules and ta through its `a` list entry (CheckDottedAbbrev,
// translateword.c:316, then SpeakIndividualLetters).  Expected values are what C espeak-ng
// master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 6] = [
    ("mk", "A.B.C.", "ˌæbˌətsˈə", ",&b,@ts'@"),
    ("mk", "U.S.A.", "ˌʊsˌəˈa", ",Us,@'a"),
    ("mk", "e.g. this", "ˌɛɡˈə thˈis", ",Eg'@ th'is"),
    ("ta", "A.B.C.", "ˈeːjbˌiːsˌiː", "'e:jb,i:s,i:"),
    ("mk", "a.b.c", "ˌæbˌətsˈə", ",&b,@ts'@"),
    ("ta", "a.b.c", "ˈeːjbˌiːsˌiː", "'e:jb,i:s,i:"),
];

#[test]
fn voice_that_names_the_letters_reads_the_run() {
    for (lang, text, ipa, espeak) in CASES {
        assert_eq!(run(lang, "--ipa", text), ipa, "{lang} --ipa {text}");
        assert_eq!(run(lang, "-x", text), espeak, "{lang} -x {text}");
    }
}
