// Tag-ended clauses carry `CLAUSE_INTONATION_NONE`, so their last word gets no tonic.
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

type Case = (&'static str, &'static str, &'static str);

fn check(cases: &[Case]) {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
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

#[test]
fn break_ends_a_clause_without_a_tonic() {
    let one_two = ("wˌɒn\ntˈuː\n", "w,0n\nt'u:\n");
    check(&[
        ("one <break/> two", one_two.0, one_two.1),
        ("one <break strength=\"medium\"/> two", one_two.0, one_two.1),
        ("one <break strength=\"strong\"/> two", one_two.0, one_two.1),
        ("one <break time=\"500ms\"/> two", one_two.0, one_two.1),
        ("one <break strength=\"weak\"/> two", "wˈɒn tˈuː\n", "w'0n t'u:\n"),
        ("hello <break/>", "həlˈəʊ\n\n", "h@l'oU\n\n"),
    ]);
}

fn samples(text: &str) -> usize {
    let path = std::env::temp_dir().join(format!("ssml_emph_{}_{}.wav", std::process::id(), text.len()));
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-m", "-v", "en", "-w"])
        .arg(&path)
        .arg(text)
        .status()
        .expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav").len();
    let _ = std::fs::remove_file(&path);
    bytes.saturating_sub(44) / 2
}

/// `<emphasis>` words take stress 6 (C `ChangeWordStress(..., 6)`), which the pitch module
/// reads; sample counts are `(bytes - 44) / 2` of `espeak-ng -m -v en -w`.
#[test]
fn emphasized_words_take_stress_six() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases = [
        ("a <emphasis>big</emphasis> dog", 21720),
        ("<emphasis>x</emphasis>", 14839),
        ("<emphasis level=\"strong\">dog</emphasis>", 14065),
        ("<emphasis>no</emphasis>", 14898),
        ("say <emphasis>no</emphasis>", 20409),
        ("<emphasis>big dog</emphasis> runs", 27232),
        ("<emphasis level=\"reduced\">big dog</emphasis>", 18710),
    ];
    let bad: Vec<String> = cases
        .iter()
        .filter_map(|&(t, want)| {
            let got = samples(t);
            (got != want).then(|| format!("{t:?}: got {got}, C {want}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

#[test]
fn emphasis_phonemes_follow_c() {
    check(&[
        ("say <emphasis>no</emphasis>", "sˌeɪ nˈəʊ\n", "s,eI_! n'oU\n"),
        ("<emphasis><s>hello</s></emphasis>", "\nhəlˈəʊ\n\n", "\n_!h@l'oU\n\n"),
        ("<emphasis>big dog</emphasis> runs", "bˈɪɡ dˈɒɡ ɹˈʌnz\n", "_!b'Ig_! d'0g r'Vnz\n"),
    ]);
}

#[test]
fn prosody_does_not_end_the_clause() {
    check(&[("<prosody pitch=\"high\">up there</prosody>", "ˌʌp ðˈeə\n", ",Vp D'e@\n")]);
}
