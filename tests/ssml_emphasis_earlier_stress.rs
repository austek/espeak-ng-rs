// A word C spells letter by letter (`spell_word`) returns before the emphasis stress change,
// so it never becomes stress 6 and an earlier primary stress stays.
// Expected values come from C espeak-ng master (f0fb11a8): whole stdout of
// `espeak-ng -q -m -v en --ipa|-x <text>`.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-m", "-v", "en", flag, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn spelled_words_do_not_lower_earlier_stress() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases = [
        ("<speak>five <emphasis>PM</emphasis> now</speak>", "fˈaɪv pˌiːˈɛm nˈaʊ\n\n", "f'aIv_! p,i:;'Em n'aU\n\n"),
        ("<speak>five <emphasis>XQ</emphasis></speak>", "fˈaɪv ˌɛkskjˈuː\n\n", "f'aIv_! ,Ekskj'u:\n\n"),
        ("<speak>five <emphasis>USA</emphasis></speak>", "fˈaɪv jˌuːˌɛsˈeɪ\n\n", "f'aIv_! j,u:,Es'eI\n\n"),
        ("<speak>five <emphasis>pm</emphasis></speak>", "fˌaɪv pˌiːˈɛm\n\n", "f,aIv_! p,i:;'Em\n\n"),
    ];
    let bad: Vec<String> = cases
        .iter()
        .flat_map(|&(text, ipa, x)| [("--ipa", ipa), ("-x", x)].map(move |(f, want)| (text, f, want)))
        .filter_map(|(text, flag, want)| {
            let got = run(flag, text);
            (got != want).then(|| format!("{flag} {text:?}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
