// A `$brk` word puts a short pause before itself, except as the last word of a
// clause.  Expected mnemonics are from C espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("nl_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", "nl", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', " "))
}

#[test]
fn brk_words_lose_their_pause_at_the_end_of_a_clause() {
    let cases = [
        ("die", "d'i"),
        ("met", "m'Et"),
        ("door", "d'o:r"),
        ("te", "t'@"),
        ("op", "_|'Op"),
        ("aan.", "_|'a:n"),
        ("ik zie die", "_|Ik z'i di"),
        ("hij kijkt naar de ster in", "hEI k'EIkt na:r d@ st'Er _|'In"),
        ("kijk, op", "k'EIk _|'Op"),
        ("die man", "_!di m'An"),
        ("met hem", "_!mEt h'Em"),
        ("in huis", "_!_|In h'Wys"),
    ];
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no nl data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
