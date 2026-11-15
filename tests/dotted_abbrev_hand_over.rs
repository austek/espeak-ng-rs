// tests/dotted_abbrev_hand_over.rs
//
// When the first letter of a dotted run belongs to another voice, C joins the run into the
// buffer and the other voice translates the clause again from the first word.  The words that
// would have been skipped now read what the joined letters left under their start offsets
// (`x . y . z`: `xyz`, then `z` under the first dot).  Expected values are what C espeak-ng
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

const CASES: [Case; 16] = [
    ("ru", "x . y . z", "(en)zˈaɪz zˈɛd(ru)", "(en)z'aIz z'Ed(ru)"),
    ("ru", "a . b . c done", "(en)ˌeɪbˌiːsˈiː sˈiː dˈʌn(ru)", "(en),eIb,i:s'i: s'i: d'Vn(ru)"),
    ("ru", "A .B .C", "(en)ˌeɪbˌiːsˈiː sˈiː(ru)", "(en),eIb,i:s'i: s'i:(ru)"),
    (
        "ru",
        "a . b . c . d . e . f . g done",
        "(en)ɐbkdˈɛfɡ sˈiːdˈɛfɡ ˈɛfɡ dʒˈiː dˈʌn(ru)",
        "(en)a#bkd'Efg s'i:d'Efg 'Efg dZ'i: d'Vn(ru)",
    ),
    ("uk", "x . y . z", "(en)zˈaɪz zˈɛd(uk)", "(en)z'aIz z'Ed(uk)"),
    ("bg", "a . b . c done", "(en)ˌeɪbˌiːsˈiː sˈiː dˈʌn(bg)", "(en),eIb,i:s'i: s'i: d'Vn(bg)"),
    ("ar", "x . y . z", "(en)zˈaɪz zˈɛd(ar)", "(en)z'aIz z'Ed(ar)"),
    ("zh", "x . y . z", "(en)zˈaɪ5z zˈɛ5d(cmn)", "(en)z'aI55z_| z'E55d(cmn)_|"),
    (
        "zh",
        "a . b . c done",
        "(en)ˌeɪ5bˌiː5sˈiː5 sˈiː5 dˈʌ5n(cmn)",
        "(en),eI55b,i:55s'i:55_| s'i:55_| d'V55n(cmn)_|",
    ),
    ("hi", "x . y . z", "(en)zˈaɪz zˈɛd(hi)", "(en)z'aIz z'Ed(hi)"),
    ("el", "a . b . c done", "(en)ˌeɪbˌiːsˈiː sˈiː dˈʌn(el)", "(en),eIb,i:s'i: s'i: d'Vn(el)"),
    ("ko", "x . y . z", "(en)zˈaɪz zˈɛd(ko)", "(en)z'aIz z'Ed(ko)"),
    ("he", "a . b . c done", "(en)ˌeɪbˌiːsˈiː sˈiː dˈʌn(he)", "(en),eIb,i:s'i: s'i: d'Vn(he)"),
    ("fa", "x . y . z", "(en)zˈaɪz zˈɛd(fa)", "(en)z'aIz z'Ed(fa)"),
    ("th", "a . b . c done", "(en)ˌeɪbˌiːsˈiː sˈiː dˈʌn(th)", "(en),eIb,i:s'i: s'i: d'Vn(th)"),
    ("am", "x . y . z", "(en)zˈaɪz zˈɛd(am)", "(en)z'aIz z'Ed(am)"),
];

#[test]
fn handed_over_run_leaves_its_tail_for_the_second_pass() {
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
