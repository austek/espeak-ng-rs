// C's LookupNum3 puts an end-of-word marker in front of the tens and units of
// a number, so the break that SetWordStress writes before a vowel-initial
// number is followed by a word start: `-x` prints a space after it.
// Expected mnemonics are from C espeak-ng master.

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

fn check(cases: &[(&str, &str)]) {
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no nl data");
            return;
        };
        assert_eq!(got, *want, "{text}");
    }
}

#[test]
fn vowel_initial_small_numbers_space_the_break() {
    check(&[
        ("1", "_! 'e:n"),
        ("8", "_! 'Axt"),
        ("11", "_! 'Elf"),
        ("21", "_! 'e:nEntv#,Int@x"),
        ("88", "_! 'AxtEnt,Axt@x"),
    ]);
}

#[test]
fn numbers_with_hundreds_or_thousands_keep_the_break_attached() {
    check(&[("8000", "_!'Ax! t,WyzEnt"), ("1000", "d'WyzEnt"), ("2", "tv#'e:")]);
}

#[test]
fn vowel_initial_grouped_numbers_space_the_break() {
    check(&[
        ("1000000", "_! 'e:n m,iljun"),
        ("8000000", "_! 'Axt m,iljun"),
        ("18000", "_! 'Axtin d,WyzEnt"),
        ("21000", "_! 'e:nEntv#,Int@x d,WyzEnt"),
    ]);
}

#[test]
fn later_groups_follow_the_same_rule() {
    check(&[
        ("1000001", "_! 'e:n m,iljun_! _! _! 'e:n"),
        ("10001", "t'in d,WyzEnt_! _! 'e:n"),
        ("801001", "_!'Axth,Ond@rt ,e:n d,WyzEnt_! _! 'e:n"),
        ("88888888", "_! 'AxtEnt,Axt@x m,iljun_! _!'Axth,Ond@rt ,AxtEnt,Axt@x d,WyzEnt_! _!'Axth,Ond@rt ,AxtEnt,Axt@x"),
    ]);
}

#[test]
fn vowel_initial_ordinals_space_the_break() {
    check(&[("1e", "_! 'I:rst@"), ("8e", "_! 'Axtst@"), ("21e", "_! 'e:nEntv#,Int@xst@")]);
}
