// `-m` phoneme output read clause by clause, against C espeak-ng master (f0fb11a8).
// Expected values are the whole stdout of `espeak-ng -q -m -v en --ipa|-x <text>`:
// C prints one line per clause, empty clauses included.

use std::process::Command;

fn run(flag: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-m", "-v", "en", flag, text])
        .output()
        .expect("run");
    String::from_utf8_lossy(&out.stdout).to_string()
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

#[test]
fn block_tags_end_clauses_and_leave_empty_lines() {
    check(&[
        ("<h1>Title</h1>body", "\ntˈaɪtəl\nbˈɒdi\n", "\nt'aIt@L\nb'0di\n"),
        ("<h1>Title</h1>", "\ntˈaɪtəl\n\n", "\nt'aIt@L\n\n"),
        ("<ul><li>one</li><li>two</li></ul>", "\nwˈɒn\n\ntˈuː\n\n", "\nw'0n\n\nt'u:\n\n"),
        ("<s>One</s><s>Two</s>", "\nwˈɒn\n\ntˈuː\n\n", "\nw'0n\n\nt'u:\n\n"),
        ("hello <br/>", "həlˈəʊ\n\n", "h@l'oU\n\n"),
    ]);
}

#[test]
fn speak_tags_leave_an_empty_closing_clause() {
    check(&[
        ("<speak></speak>", "\n\n", "\n\n"),
        ("<speak>hello?</speak>", "həlˈəʊ\n\n\n", "h@l'oU\n\n\n"),
        ("<p>!</p>", "\nˈɛkskləmˌeɪʃən\n\n", "\n_:'Ekskl@m,eIS@n_:\n\n"),
    ]);
}

#[test]
fn audio_tags_end_clauses() {
    check(&[(
        "Hello <audio src=\"nofile.wav\">alt text</audio> world",
        "həlˈəʊ\nˈɒlt tˈɛkst\nwˈɜːld\n",
        "h@l'oU\n'0lt t'Ekst\nw'3:ld\n",
    )]);
}

#[test]
fn voice_changes_switch_after_the_clause_that_ends_them() {
    check(&[
        (
            "<speak>hello <voice xml:lang=\"fr\">bonjour</voice> world</speak>",
            "həlˈəʊ\nbɔ̃ʒˈuʁ\nwˈɜːld\n\n",
            "h@l'oU\nbO~Z'ur\nw'3:ld\n\n",
        ),
        ("<speak xml:lang=\"fr\">bonjour</speak>", "\nbɔ̃ʒˈuʁ\n\n", "\nbO~Z'ur\n\n"),
        ("<voice xml:lang=\"en\">hello</voice> there", "həlˈəʊ ðˈeə\n", "h@l'oU D'e@\n"),
        ("<voice xml:lang=\"de\"></voice>", "\n\n\n", "\n\n\n"),
    ]);
}

/// C quirk: `</s>` pops a by-value copy of the stack depth, so the language stays German.
#[test]
fn closing_sentence_keeps_its_language() {
    check(&[("<s xml:lang=\"de\">hallo</s> hello", "\nhˈaloː\nhˈɛloː\n", "\nh'alo:\nh'Elo:\n")]);
}

/// A full stop the reader keeps in the clause is silent when a space follows it (C
/// `TranslateClause` removes a dot after a word).
#[test]
fn full_stop_kept_inside_a_clause_is_silent() {
    check(&[
        ("Hello. <mark name=\"x\"/>world", "həlˈəʊ wˈɜːld\n", "h@l'oU w'3:ld\n"),
        ("Hello.<mark name=\"x\"/> World", "həlˈəʊ\nwˈɜːld\n", "h@l'oU\nw'3:ld\n"),
    ]);
}
