// tests/ssml_sub_alias_audio.rs
//
// Byte-for-byte WAV parity with C espeak-ng master (f0fb11a8) for SSML `<sub alias>`.  The
// alias text is copied into the clause without `charix`, so C's `charix_top` stops at the
// last character the reader read: a trailing alias word has no source position and joins the
// word before it.  Each expected value is the FNV-1a 64 hash of the PCM samples the C binary
// writes, plus its sample count.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-ssa-{}-{n}.wav", std::process::id()));
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(args)
        .arg("-w")
        .arg(&out)
        .arg(text)
        .status()
        .expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&out).expect("read wav");
    let _ = std::fs::remove_file(&out);
    bytes[44..].to_vec()
}

type Case = (&'static [&'static str], &'static str, usize, u64);

fn check(cases: &[Case]) {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = cases
        .iter()
        .filter_map(|&(args, text, n, hash)| {
            let pcm = render(args, text);
            (pcm.len() / 2 != n || fnv1a(&pcm) != hash)
                .then(|| format!("{args:?} {text:?}: {} samples (C {n})", pcm.len() / 2))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

/// The alias is the last text of the clause or only a mark follows it.
#[test]
fn alias_at_the_clause_end() {
    check(&[
        (&["-m", "-v", "en"], "<speak>a <sub alias=\"foo\">x</sub><mark name=\"m\"/></speak>", 15258, 0xec9724bf166c82e9),
        (&["-m", "-v", "en"], "<speak><sub alias=\"foo\">x</sub><mark name=\"m\"/></speak>", 13650, 0x2100ff95a347155a),
        (&["-m", "-v", "en"], "<speak>a <sub alias=\"foo\">x</sub></speak>", 15258, 0xec9724bf166c82e9),
        (&["-m", "-v", "en"], "<speak>a <sub alias=\"foo bar\">x</sub><mark name=\"m\"/></speak>", 20504, 0xcd3c57f81657b8b6),
    ]);
}

#[test]
fn alias_before_other_markup() {
    check(&[
        (&["-m", "-v", "en"], "<speak>a <sub alias=\"foo\">x</sub> <mark name=\"m\"/> b</speak>", 20554, 0x3f2366b88d504deb),
        (&["-m", "-v", "en"], "<speak>a <sub alias=\"foo\">x</sub><break time=\"100ms\"/></speak>", 14063, 0x4dc722320ad65b17),
        (&["-m", "-v", "en"], "<speak>a <sub alias=\"foo\">x</sub><voice name=\"de\">welt</voice></speak>", 22947, 0xa06d096614b1aded),
        (&["-m", "-v", "en"], "<speak>a <sub alias=\"foo\">x</sub><emphasis>b</emphasis></speak>", 20788, 0x9e6411bb426118de),
        (&["-m", "-v", "en"], "<speak>a <sub alias=\"foo\">x</sub> b <sub alias=\"bar\">y</sub><mark name=\"m\"/></speak>", 25446, 0x4cb69088eebbabd1),
    ]);
}

fn phonemes(flag: &str, lang: &str, text: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli")).args(["-q", flag, "-m", "-v", lang, text]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// C writes no space before a word whose `sourceix` is 0.
#[test]
fn alias_at_the_clause_end_starts_no_word_in_the_phoneme_output() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let text = "<speak>a <sub alias=\"foo\">x</sub></speak>";
    assert_eq!(phonemes("-x", "en", text), "a#f'u:\n\n");
    assert_eq!(phonemes("--ipa", "en", text), "ɐfˈuː\n\n");
    let before_a_word = "<speak>a <sub alias=\"foo\">x</sub> b</speak>";
    assert_eq!(phonemes("-x", "en", before_a_word), "a# f'u: b'i:\n\n");
}

#[test]
fn alias_in_other_languages() {
    check(&[
        (&["-m", "-v", "de"], "<speak>a <sub alias=\"Welt\">x</sub><mark name=\"m\"/></speak>", 19085, 0xf23865eaa8ddd656),
        (&["-m", "-v", "ku"], "<speak>roj <sub alias=\"baş\">x</sub><mark name=\"m\"/></speak>", 20293, 0x33d9b9ebc930d64b),
        (&["-m", "-v", "ru"], "<speak>мир <sub alias=\"привет\">x</sub><mark name=\"m\"/></speak>", 21078, 0x862ae92951ba7b71),
    ]);
}
