// tests/dotted_abbrev_unspaced_hand_over.rs
//
// A dotted run written without blanks (`a.b.c`, `U.S.A. today`) reaches CheckDottedAbbrev the
// same way as `a . b . c`, so when another voice takes the run over, the words the joined
// letters left behind are read too (translateword.c:1176).  Expected values are what C
// espeak-ng master (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: [Case; 10] = [
    ("ru", "a.b.c", "(en)ˌeɪbˌiːsˈiː sˈiː(ru)", "(en),eIb,i:s'i: s'i:(ru)"),
    ("ru", "U.S.A. today", "(en)jˌuːˌɛsˈeɪ ˈeɪ tədˈeɪ(ru)", "(en)j,u:,Es'eI 'eI t@d'eI(ru)"),
    ("ru", "z.z.z.z", "(en)zˌɛdzˌɛdzˌɛdzˈɛd zˌɛdzˈɛd(ru)", "(en)z,Edz,Edz,Edz'Ed z,Edz'Ed(ru)"),
    ("ru", "w.w.w", "(en)dˌʌbəljˌuːdˌʌbəljˌuːdˈʌbəljˌuː dˈʌbəljˌuː(ru)", "(en)d,Vb@Lj,u:d,Vb@Lj,u:d'Vb@Lj,u: d'Vb@Lj,u:(ru)"),
    ("uk", "a.b.c", "(en)ˌeɪbˌiːsˈiː sˈiː(uk)", "(en),eIb,i:s'i: s'i:(uk)"),
    ("el", "U.S.A. today", "(en)jˌuːˌɛsˈeɪ ˈeɪ tədˈeɪ(el)", "(en)j,u:,Es'eI 'eI t@d'eI(el)"),
    ("ar", "a.b.c", "(en)ˌeɪbˌiːsˈiː sˈiː(ar)", "(en),eIb,i:s'i: s'i:(ar)"),
    ("hi", "a.b.c", "(en)ˌeɪbˌiːsˈiː sˈiː(hi)", "(en),eIb,i:s'i: s'i:(hi)"),
    ("zh", "a.b.c", "(en)ˌeɪ5bˌiː5sˈiː5 sˈiː5(cmn)", "(en),eI55b,i:55s'i:55_| s'i:55(cmn)_|"),
    ("ko", "U.S.A. today", "(en)jˌuːˌɛsˈeɪ ˈeɪ tədˈeɪ(ko)", "(en)j,u:,Es'eI 'eI t@d'eI(ko)"),
];

#[test]
fn unspaced_dotted_run_reads_the_words_it_leaves() {
    for (lang, text, ipa, espeak) in CASES {
        assert_eq!(run(lang, "--ipa", text), ipa, "{lang} --ipa {text}");
        assert_eq!(run(lang, "-x", text), espeak, "{lang} -x {text}");
    }
}
