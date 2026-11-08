// A `$brk` word (bei, nach, nur, ...) gets a pause before it, except as the
// last word of a clause. Expected strings come from C espeak-ng master (f0fb11a).

use std::path::Path;
use std::process::Command;

fn mnemonics(lang: &str, text: &str) -> String {
    let bin = env!("CARGO_BIN_EXE_espeak_cli");
    let out = Command::new(bin)
        .args(["-q", "-x", "-v", lang, "--", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn check(lang: &str, cases: &[(&str, &str)]) {
    if !Path::new(&format!("espeak-ng-data/{lang}_dict")).exists() {
        eprintln!("[SKIP] no {lang} data");
        return;
    }
    for (text, want) in cases {
        assert_eq!(&mnemonics(lang, text), want, "{lang} {text:?}");
    }
}

#[test]
fn brk_word_last_in_clause_has_no_pause() {
    check(
        "de",
        &[
            ("bei", "b'aI"),
            ("nach", "n'A:x"),
            ("ich nach", "_|IC n'A:x"),
            ("gehen nach", "g'e:@n n'A:x"),
            ("gehen nach.", "g'e:@n n'A:x"),
            ("ich nur", "_|IC n'u:r"),
            ("ich neben", "_|IC n'e:b@n"),
        ],
    );
}

#[test]
fn brk_word_before_another_word_keeps_its_pause() {
    check(
        "de",
        &[
            ("gehen nach dir", "g'e:@n_! nA:x d'i:r"),
            ("er gehen", "_!_|Er g'e:@n"),
            ("ich neben dir", "_|IC_! n,e:b@n d'i:r"),
        ],
    );
}

#[test]
fn swahili_brk_word_follows_the_same_rule() {
    check("sw", &[("habari za", "hab'aRi za"), ("habari za sana", "hab'aRi_! za s'ana")]);
}
