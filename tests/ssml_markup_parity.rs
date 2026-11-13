// tests/ssml_markup_parity.rs
//
// `-m` phoneme output against C espeak-ng master (f0fb11a8).  Expected values
// are what the C binary prints for `--ipa` and `-x`.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// `(input, --ipa, -x)` as printed by C.
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

/// `ignore_text` is one flag: any closing `sub`/`metadata`/`script`/`style` clears it.
#[test]
fn ignored_text_elements_and_sub_flag() {
    check(&[
        ("<metadata>foo</metadata> bar", "bˈɑː", "b'A@"),
        ("a <script> c </script> d", "ɐ dˈiː", "a# d'i:"),
        (
            "<sub alias=\"A B\">xx <metadata>q</metadata> zz</sub> y",
            "ɐ bˈiː zˌɛdzˈɛd wˈaɪ",
            "a# b'i: z,Edz'Ed w'aI",
        ),
    ]);
}

/// C writes a space before every tag except the inline HTML ones (`b`, `i`, `em`, ...).
#[test]
fn tags_separate_words_except_inline_html() {
    check(&[
        ("one<foo>two</foo>three", "wˈɒn tˈuː θɹˈiː", "w'0n t'u: Tr'i:"),
        ("one<!-- c -->two", "wˈɒn tˈuː", "w'0n t'u:"),
        ("one<b>two</b>three", "wˈɒntwəθɹˌiː", "w'0ntw@Tr,i:"),
    ]);
}

/// `<emphasis>` at level moderate or stronger puts a pause before each word inside it.
#[test]
fn emphasis_pauses_before_its_words() {
    check(&[
        ("a <emphasis>b", "ɐ bˈiː", "a#_! b'i:"),
        ("a <emphasis level=\"strong\">b c</emphasis> d", "ɐ bˈiː sˈiː dˈiː", "a#_! b'i:_! s'i: d'i:"),
        ("a <emphasis level=\"reduced\">b</emphasis> c", "ɐ bˈiː sˈiː", "a# b'i: s'i:"),
        ("<emphasis>hello</emphasis>", "həlˈəʊ", "_!h@l'oU"),
    ]);
}

fn run_raw(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-m", text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// A tag that ends a clause at the very end of the text leaves one more, empty, clause line.
#[test]
fn trailing_clause_tag_adds_an_empty_clause_line() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        return;
    }
    let cases: &[(&str, &str, &str)] = &[
        ("One</s>", "w\u{2c8}\u{252}n\n\n", "w'0n\n\n"),
        ("<s>One</s>", "\nw\u{2c8}\u{252}n\n\n", "\nw'0n\n\n"),
        ("One<br>", "w\u{2c8}\u{252}n\n\n", "w'0n\n\n"),
        ("One</speak> ", "w\u{2c8}\u{252}n\n\n", "w'0n\n\n"),
        ("One<p>", "w\u{2c8}\u{252}n\n\n", "w'0n\n\n"),
        ("One<break strength=\"weak\"/>", "w\u{2c8}\u{252}n\n", "w'0n\n"),
        ("One<emphasis>", "w\u{2c8}\u{252}n\n", "w'0n\n"),
    ];
    let bad: Vec<String> = cases
        .iter()
        .flat_map(|&(t, ipa, x)| [("--ipa", ipa, t), ("-x", x, t)])
        .filter_map(|(f, want, t)| {
            let got = run_raw(f, t);
            (got != want).then(|| format!("{f} {t:?}: got {got:?}, C {want:?}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

/// Only `alphabet="espeak"` emits `[[ph]]`; the element content is read either way.
#[test]
fn phoneme_needs_the_espeak_alphabet_and_keeps_its_content() {
    check(&[
        ("<phoneme alphabet=\"espeak\" ph=\"k\">see</phoneme> now", "k sˈiː nˈaʊ", "k s'i: n'aU"),
        ("<phoneme alphabet=\"ipa\" ph=\"k\">see</phoneme> now", "sˈiː nˈaʊ", "s'i: n'aU"),
        ("say <phoneme ph=\"k\">see</phoneme> now", "sˈeɪ sˈiː nˈaʊ", "s'eI s'i: n'aU"),
    ]);
}

/// C starts a tag only when `<` is followed by `/`, a letter, `!` or `?`.
#[test]
fn lt_before_a_non_tag_character_is_literal() {
    check(&[
        ("a < b", "ɐ bˈiː", "a#_:_: b'i:"),
        ("1 < 2", "wˈɒn tˈuː", "w'0n_:_: t'u:"),
        ("3 < 4 and 5 > 2", "θɹˈiː fˈɔː and fˈaɪv tˈuː", "Tr'i:_:_: f'o@_:_: and f'aIv_:_: t'u:"),
    ]);
}
