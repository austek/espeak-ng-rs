// tests/ssml_voice_change_audio.rs
//
// WAV parity with C espeak-ng master (f0fb11a8) for SSML `<voice>` changes.  A change from a
// base voice that has a `+variant` keeps that variant (`base_voice_variant_name`), and the
// reader compares the new voice with the identifier of the voice that is loaded, so a
// `<voice>` that names the current voice is no change.  Each expected value is the FNV-1a 64
// hash of the PCM samples the C binary writes, plus its sample count.  The female variants
// (`f2`, `f3`) have a roughness that C draws from `rand()`, so their runs only agree in length.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-svca-{}-{n}.wav", std::process::id()));
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

fn have_data() -> bool {
    Path::new("espeak-ng-data/en_dict").exists()
}

fn check(cases: &[Case]) {
    if !have_data() {
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

fn check_length(cases: &[(&'static [&'static str], &'static str, usize)]) {
    if !have_data() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = cases
        .iter()
        .filter_map(|&(args, text, n)| {
            let got = render(args, text).len() / 2;
            (got != n).then(|| format!("{args:?} {text:?}: {got} samples (C {n})"))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}

#[test]
fn language_and_name_changes_from_en() {
    check(&[
        (&["-m", "-v", "en"], "<speak>Hello <voice name=\"de\">welt</voice> end.</speak>", 30054, 0x8ed0eb437d461c5e),
        (&["-m", "-v", "en"], "<speak>Hello <voice xml:lang=\"de\">welt</voice> end.</speak>", 30054, 0x8ed0eb437d461c5e),
        (&["-m", "-v", "en"], "<speak>Hello <voice name=\"ku\">roj</voice> end.</speak>", 29584, 0xa09ffc1206a8f1a0),
        (&["-m", "-v", "en"], "<speak>Hello <voice xml:lang=\"ru\">мир</voice> end.</speak>", 26266, 0xa63dac10e29d22cf),
    ]);
}

#[test]
fn language_and_name_changes_from_other_bases() {
    check(&[
        (&["-m", "-v", "de"], "<speak>Hallo <voice xml:lang=\"en\">world</voice> Ende.</speak>", 29941, 0xec2a7fcc04c865b8),
        (&["-m", "-v", "de"], "<speak>Hallo <voice name=\"ru\">мир</voice> Ende.</speak>", 25962, 0xd2344d3b36573959),
        (&["-m", "-v", "ku"], "<speak>Roj <voice name=\"en\">hello</voice> baş.</speak>", 30295, 0xff8da11d486dc733),
        (&["-m", "-v", "ku"], "<speak>Roj <voice xml:lang=\"de\">hallo</voice> baş.</speak>", 28264, 0x21f4b74401dffda3),
        (&["-m", "-v", "ru"], "<speak>Привет <voice xml:lang=\"en\">hello</voice> мир.</speak>", 30716, 0xa438af291ce38fdb),
        (&["-m", "-v", "ru"], "<speak>Привет <voice name=\"de\">hallo</voice> мир.</speak>", 28690, 0xd67f017d3da31030),
    ]);
}

#[test]
fn gender_changes_without_a_female_variant() {
    check(&[
        (&["-m", "-v", "en"], "<speak>Hello <voice gender=\"male\">world</voice> end.</speak>", 26830, 0xeb02f58e6633bc81),
        (&["-m", "-v", "en"], "<speak>Hello <voice gender=\"neutral\">world</voice> end.</speak>", 26830, 0xeb02f58e6633bc81),
        (&["-m", "-v", "en"], "<speak>Hello <voice xml:lang=\"de\" gender=\"male\">welt</voice> end.</speak>", 30054, 0x8ed0eb437d461c5e),
    ]);
}

#[test]
fn female_gender_selects_the_f2_variant() {
    check_length(&[
        (&["-m", "-v", "en"], "<speak>Hello <voice gender=\"female\">world</voice> end.</speak>", 31767),
        (&["-m", "-v", "de"], "<speak>Hallo <voice gender=\"female\">welt</voice> Ende.</speak>", 31442),
        (&["-m", "-v", "ku"], "<speak>Roj <voice gender=\"female\">hello</voice> baş.</speak>", 34363),
        (&["-m", "-v", "en"], "<speak>Hello <voice gender=\"female\" xml:lang=\"de\">welt</voice> end.</speak>", 31768),
        (&["-m", "-v", "en+f2"], "<speak>Hello <voice xml:lang=\"de\">welt</voice> end.</speak>", 33646),
    ]);
}

#[test]
fn a_change_keeps_the_base_variant() {
    check(&[
        (&["-m", "-v", "en+m3"], "<speak>Hello <voice xml:lang=\"de\">welt</voice> end.</speak>", 28923, 0x12a63bd09eef5370),
        (&["-m", "-v", "en+m2"], "<speak>Hello <voice xml:lang=\"de\">welt</voice> end.</speak>", 31823, 0xc2949daa49f8a61d),
        (&["-m", "-v", "en+f1"], "<speak>Hello <voice xml:lang=\"de\">welt</voice> end.</speak>", 32312, 0xff7f1017bd0565a9),
        (&["-m", "-v", "en+f4"], "<speak>Hello <voice xml:lang=\"ru\">мир</voice> end.</speak>", 31075, 0x927400501ac09afb),
        (&["-m", "-v", "en+croak"], "<speak>Hello <voice xml:lang=\"de\">welt</voice> end.</speak>", 30157, 0x7d9e2c6062ff0f49),
        (&["-m", "-v", "en+m3"], "<speak>Hello <voice name=\"de\">welt</voice> end.</speak>", 28923, 0x12a63bd09eef5370),
        (&["-m", "-v", "ru+m2"], "<speak>Привет <voice xml:lang=\"en\">hello</voice> мир.</speak>", 33021, 0x4ce35e6caa837d23),
        (&["-m", "-v", "de+m1"], "<speak>Hallo <voice xml:lang=\"ku\">roj</voice> Ende.</speak>", 28946, 0xde175b4534d60d81),
        (&["-m", "-v", "ku+m3"], "<speak>Roj <voice xml:lang=\"en\">hello</voice> baş.</speak>", 28832, 0xb9b06e073afe6877),
    ]);
}

/// A `<voice>` that names the voice already loaded starts no new voice and no clause.
#[test]
fn naming_the_loaded_voice_is_no_change() {
    check(&[
        (&["-m", "-v", "en+m3"], "<speak>Hello <voice gender=\"male\">world</voice> end.</speak>", 25933, 0x228a1e1e5d7c2661),
    ]);
}
