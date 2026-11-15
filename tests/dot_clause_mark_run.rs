// tests/dot_clause_mark_run.rs
//
// C `ReadClause` tests the bits of the first clause mark against `CLAUSE_QUESTION |
// CLAUSE_EXCLAMATION` (readclause.c:794).  Every clause type shares bits with that mask, so any
// run of clause marks collapses to its first mark and the rest never reach the clause text.  A
// dot followed by `!`, `:`, `,`, `…` or `–` therefore keeps the dot and loses the mark, and the
// clause ends only where the first mark is followed by a blank.  Expected values are what C
// espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 16] = [
    ("en", "one.!two", "wˈɒn dˈɒt tˈuː", "w'0n d'0t t'u:"),
    ("en", "one.:two", "wˈɒn dˈɒt tˈuː", "w'0n d'0t t'u:"),
    ("en", "one.!!two", "wˈɒn dˈɒt tˈuː", "w'0n d'0t t'u:"),
    ("en", "one., two", "wˈɒn tˈuː", "w'0n t'u:"),
    ("en", "one.? two", "wˈɒn tˈuː", "w'0n t'u:"),
    ("en", "one.! two", "wˈɒn tˈuː", "w'0n t'u:"),
    ("en", "one.: two", "wˈɒn tˈuː", "w'0n t'u:"),
    ("en", "one.; two", "wˈɒn tˈuː", "w'0n t'u:"),
    ("en", "one.… two", "wˈɒn tˈuː", "w'0n t'u:"),
    ("en", "one.– two", "wˈɒn tˈuː", "w'0n t'u:"),
    ("en", "one.…two", "wˈɒn dˈɒt tˈuː", "w'0n d'0t t'u:"),
    ("en", "one.–two", "wˈɒn dˈɒt tˈuː", "w'0n d'0t t'u:"),
    ("xex", "one.… two", "ˈoːnɛ\ntwˈoː", "_!'o:nE\ntw'o:"),
    ("xex", "one.– two", "ˈoːnɛ\ntwˈoː", "_!'o:nE\ntw'o:"),
    ("mto", "one.… two", "oˈn̩e\ntʰ ˌuβe dˈoβle ˈo", "o'ne\nt# ,uBe d'oBle 'o"),
    ("ka", "one.– two", "(en)wˈɒn(ka)\n(en)tˈuː(ka)", "(en)w'0n(ka)\n(en)t'u:(ka)"),
];

#[test]
fn a_clause_mark_run_after_a_dot_collapses_to_its_first_mark() {
    let mut failures = Vec::new();
    for (lang, text, ipa, x) in CASES {
        for (flag, want) in [("--ipa", ipa), ("-x", x)] {
            let got = run(lang, flag, text);
            if got != want {
                failures.push(format!("{lang} {flag} {text:?}\n  want {want:?}\n  got  {got:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
