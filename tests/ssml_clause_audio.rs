// `-m` audio rendered clause by clause, against C espeak-ng master (f0fb11a8).
// Expected values are the sample counts of `espeak-ng -m -v en -w out.wav <text>`
// ((file bytes - 44) / 2).

use std::process::Command;

fn samples(text: &str) -> usize {
    let dir = std::env::temp_dir().join(format!("ssml_clause_audio_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", text.len() * 131 + text.bytes().map(usize::from).sum::<usize>()));
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

fn check(cases: &[(&str, usize)]) {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = cases
        .iter()
        .filter_map(|&(text, want)| {
            let got = samples(text);
            (got != want).then(|| format!("{text:?}: got {got}, C {want}"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

/// Block tags end clauses with their own pauses; the clause after the last tag is empty
/// and only tops up the pause.
#[test]
fn block_tags_end_clauses_with_their_pauses() {
    check(&[
        ("hello <br/>", 14298),
        ("<h1>Title</h1>", 33659),
        ("<s>One</s><s>Two</s>", 44484),
        ("<ul><li>one</li><li>two</li></ul>", 29556),
        ("<p>Para one</p><p>Para two</p>", 59570),
    ]);
}

/// Empty clauses still render their pause, ending in the 10 ms end-of-text pause.
#[test]
fn empty_clauses_render() {
    check(&[("<speak></speak>", 6791), ("<voice xml:lang=\"de\"></voice>", 154), ("<speak>Hello</speak>", 15952)]);
}

#[test]
fn audio_tags_end_clauses() {
    check(&[("Hello <audio src=\"nofile.wav\">alt text</audio> world", 37490)]);
}

#[test]
fn breaks_keep_their_length() {
    check(&[("one <break time=\"500ms\"/> two", 30192)]);
}
