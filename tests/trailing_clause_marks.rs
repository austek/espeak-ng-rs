// Clause marks left after the last clause: C reports one empty clause (an empty
// line) for any mark, a spoken name for `!` and `:`, and nothing for a leading
// `:)`. Expected stdout is from C espeak-ng master f0fb11a8.

use std::path::Path;
use std::process::Command;

fn run(flag: &str, lang: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// (voice, text, `--ipa` stdout, `-x` stdout)
const CASES: &[(&str, &str, &str, &str)] = &[
    ("en", "hello, ,", "həlˈəʊ\n\n", "h@l'oU\n\n"),
    ("en", "hello; ;", "həlˈəʊ\n\n", "h@l'oU\n\n"),
    ("en", "hello? ?", "həlˈəʊ\n\n", "h@l'oU\n\n"),
    ("en", "hello . ,", "həlˈəʊ\n\n", "h@l'oU\n\n"),
    ("en", "hello. .", "həlˈəʊ\n\n", "h@l'oU\n\n"),
    ("en", "hello , , ,", "həlˈəʊ\n\n", "h@l'oU\n\n"),
    ("en", "hello . . .", "həlˈəʊ\n\n", "h@l'oU\n\n"),
    ("en", "hello :(", "həlˈəʊ\n\n", "h@l'oU\n\n"),
    ("en", "hello :)", "həlˈəʊ\n\n", "h@l'oU\n\n"),
    ("en", "hello , world , ,", "həlˈəʊ\nwˈɜːld\n\n", "h@l'oU\nw'3:ld\n\n"),
    ("en", "hello! !", "həlˈəʊ\nˈɛkskləmˌeɪʃən\n", "h@l'oU\n_:'Ekskl@m,eIS@n_:\n"),
    ("en", "hello: :", "həlˈəʊ\nkˈəʊlən\n", "h@l'oU\nk'oUl@n\n"),
    ("en", ":)", "kˈəʊlən\n", "k'oUl@n\n"),
    ("de", "Hallo , ,", "hˈaloː\n\n", "h'alo:\n\n"),
    ("es", "ez , ,", "ˈeθ\n\n", "'eT\n\n"),
    ("ar", "مرحبا ، ،", "mrħbˈaː\n\n", "mRHb'a:\n\n"),
    ("en", "3, ,", "θɹˈiː\n\n", "Tr'i:\n\n"),
    ("de", "3.,", "dɾˈaɪ\n", "dr'aI\n"),
    ("en", "hello.,,", "həlˈəʊ\n", "h@l'oU\n"),
    ("en", "hello. ,\"", "həlˈəʊ\n\n", "h@l'oU\n\n"),
];

#[test]
fn marks_after_the_last_clause_give_one_empty_clause() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    for &(lang, text, ipa, x) in CASES {
        assert_eq!(run("--ipa", lang, text), ipa, "{lang} {text:?} --ipa");
        assert_eq!(run("-x", lang, text), x, "{lang} {text:?} -x");
    }
}
