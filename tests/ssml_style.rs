// `<tts:style>` punctuation and capitals, against C espeak-ng master (f0fb11a8).
// Expected values: whole stdout of `espeak-ng -q -m -v en --ipa|-x <text>`.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-m", "-v", "en", flag, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn have_data() -> bool {
    let ok = std::path::Path::new("espeak-ng-data/en_dict").exists();
    if !ok {
        eprintln!("[SKIP] no data");
    }
    ok
}

/// `(input, --ipa, -x)` as printed by C.
type Case = (&'static str, &'static str, &'static str);

fn check(cases: &[Case]) {
    if !have_data() {
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

fn samples(text: &str) -> usize {
    let dir = std::env::temp_dir().join(format!("ssml_style_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", text.bytes().map(usize::from).sum::<usize>() * 31 + text.len()));
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

#[test]
fn style_modes_match_c() {
    check(&[
        (
            r#"<tts:style field="punctuation" mode="all">a, b.</tts:style>"#,
            "ˈeɪ\nkˈɒmə\nbˈiː\ndˈɒt\n\n",
            "'eI\nk'0m@\nb'i:\nd'0t\n\n",
        ),
        (
            r#"<tts:style field="capital_letters" mode="spelling">Hello</tts:style>"#,
            "kˌapətəl həlˈəʊ\n",
            "k,ap@-t@L h@l'oU\n",
        ),
        (r#"<tts:style field="punctuation" mode="none">a, b.</tts:style>"#, "ˈeɪ\nbˈiː\n", "'eI\nb'i:\n"),
    ]);
    if have_data() {
        assert_eq!(samples(r#"<tts:style field="punctuation" mode="all">a, b.</tts:style>"#), 32859);
        assert_eq!(samples(r#"<tts:style field="capital_letters" mode="spelling">Hello</tts:style>"#), 25279);
    }
}

#[test]
fn hyphen_after_a_speed_change_pauses_like_a_spaced_dash() {
    check(&[
        (
            r#"<tts:style field="punctuation" mode="all">a -- b</tts:style>"#,
            "ɐ hˌaɪfən hˌaɪfən bˈiː\n",
            "a# h,aIf@n h,aIf@n_:_: b'i:\n",
        ),
        (
            r#"<tts:style field="punctuation" mode="all">a --- b</tts:style>"#,
            "ɐ hˌaɪfən hˌaɪfən hˌaɪfən bˈiː\n",
            "a# h,aIf@n h,aIf@n h,aIf@n_:_: b'i:\n",
        ),
    ]);
}

#[test]
fn announced_name_joins_the_word_after_a_hyphen() {
    check(&[
        (
            r#"<tts:style field="punctuation" mode="all">a-b</tts:style>"#,
            "ɐ hˌaɪfənbˈiː\n",
            "a# h,aIf@nb'i:\n",
        ),
        ("a [[h,aIf@n]]-b", "ɐ hˌaɪfənbˈiː\n", "a# h,aIf@nb'i:\n"),
    ]);
}

#[test]
fn announced_brackets_and_quotes_pause_before_the_next_word() {
    check(&[
        (
            r#"<tts:style field="punctuation" mode="all">ok (yes) "q"</tts:style>"#,
            "ˌəʊkˈeɪ lˈɛftpɐɹˌɛn jˈɛs ɹˈaɪtpɐɹˌɛn kwˈəʊts kjˈuː kwˈəʊts\n",
            ",oUk'eI l'Eftpa#r,En_: j'Es r'aItpa#r,En_: kw'oUts_: kj'u: kw'oUts\n",
        ),
        (
            r#"<tts:style field="punctuation" mode="all">x [y] {z}</tts:style>"#,
            "ˈɛks lˈɛftbɹakɪt wˈaɪ ɹˈaɪtbɹakɪt lˈɛftbɹeɪs zˈɛd ɹˈaɪtbɹeɪs\n",
            "'Eks l'EftbrakI2t_: w'aI r'aItbrakI2t_: l'EftbreIs_: z'Ed r'aItbreIs\n",
        ),
    ]);
}
