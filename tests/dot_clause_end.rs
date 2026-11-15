// tests/dot_clause_end.rs
//
// Whether a '.' set apart by blanks stays in the clause is decided by what follows it, as in
// C's ReadClause (readclause.c:817-922).  Expected values are what C espeak-ng master
// (f0fb11a8) prints.

use std::process::Command;

fn run(lang: &str, flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang])
        .arg(text)
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

type Case = (&'static str, &'static str, &'static str, &'static str);

const CASES: &[Case] = &[
    ("en", "He said .. Then left", "hiː sˈɛd\nðˈɛn lˈɛft", "hi: s'Ed\nD'En l'Eft"),
    ("en", "He said .. 5 left", "hiː sˈɛd\nfˈaɪv lˈɛft", "hi: s'Ed\nf'aIv l'Eft"),
    ("en", "He said ..", "hiː sˈɛd", "hi: s'Ed"),
    ("ar", "قال .. ثم غادر", "qˈaːl\nθˈumma ɣˈaːdar", "q'a:l\nT'umma gH'a:daR"),
    ("hi", "उसने कहा .. फिर गया", "ˈʊsnˌeː kˈʌhaː\npʰˈɪɾ ɡˈʌjaː", "'Usn,e: k'VHa:\np#'Ir g'Vja:"),
    ("fa", "گفت .. سپس رفت", "ɡˈoft\nsˈepas rˈaft", "g'oft\ns'epas R'aft"),
];

const NEWLINE_CASES: &[Case] = &[
    ("en", "He said .\nthen left", "hiː sˈɛd\nðˈɛn lˈɛft", "hi: s'Ed\nD'En l'Eft"),
    ("en", "He said .\n\nthen left", "hiː sˈɛd\nðˈɛn lˈɛft", "hi: s'Ed\nD'En l'Eft"),
    ("en", "He said .  \n  then left", "hiː sˈɛd\nðˈɛn lˈɛft", "hi: s'Ed\nD'En l'Eft"),
    ("en", "He said.\nthen left", "hiː sˈɛd\nðˈɛn lˈɛft", "hi: s'Ed\nD'En l'Eft"),
    ("de", "Er sagte .\ndann ging er", "ɛɾ zˈɑːɡtə\ndan ɡˈɪŋ ɛɾ", "_!_|Er z'A:gt@\ndan g'IN _|Er"),
    ("es", "Dijo .\nluego se fue", "dˈixo\nluˈeɣo se fwˈe", "d'ixo\nlu'eQo se fw'e"),
    ("ru", "Он сказал .\nпотом ушёл", "ˈon skazˈɑɭ\npˈotʌm uʃˈoɭ", "'on_ skaz'Al\np'otVm uS'ol"),
];

const LOWERCASE_SENTENCE_CASES: &[Case] = &[
    ("ka", "მან თქვა . მერე წავიდა.", "mˈan tʰkʰvˈa\nmˈere tsˈavida", "m'an t#k#v'a\nm'ere ts'avida"),
    ("ka", "მან თქვა. მერე წავიდა.", "mˈan tʰkʰvˈa\nmˈere tsˈavida", "m'an t#k#v'a\nm'ere ts'avida"),
    ("mto", "mi sa . ta ko.", "ˈm̩i sˈa\ntˈa kˈo", "'mi s'a\nt'a k'o"),
    ("mto", "mi sa. ta ko.", "ˈm̩i sˈa\ntˈa kˈo", "'mi s'a\nt'a k'o"),
    ("xex", "mi sa . ta ko.", "mˈi sa\ntˈa ko", "m'i sa\nt'a ko"),
    ("xex", "mi sa ta. ko ta", "mi sa tˈa\nkˈo ta", "mi sa t'a\nk'o ta"),
];

fn check(cases: &[Case]) {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = cases
        .iter()
        .flat_map(|&(l, t, i, x)| [(l, "--ipa", i, t), (l, "-x", x, t)])
        .filter_map(|(l, f, want, t)| {
            let got = run(l, f, t);
            (got != want).then(|| format!("{l} {f} {t:?}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

#[test]
fn a_spaced_dot_before_a_clause_ending_dot_is_not_read() {
    check(CASES);
}

#[test]
fn a_dot_before_a_lower_case_word_ends_the_clause_where_the_language_says_so() {
    check(LOWERCASE_SENTENCE_CASES);
}

#[test]
fn a_dot_before_a_newline_ends_the_clause_even_before_a_lower_case_word() {
    check(NEWLINE_CASES);
}
