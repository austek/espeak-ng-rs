// `TranslateClause` turns an apostrophe that is not part of a word into a blank, so a
// dictionary entry that names `'` (Catalan lists it) is never read: `s' ha` is the letter name
// `es` and `ha`, with no "accent" between them.
// Expected values come from `espeak-ng -v ca -q -x|--ipa <text>` and
// `espeak-ng -v ca -w out.wav <text>` (sample count and FNV-1a 64 hash of the PCM bytes),
// C f0fb11a8.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn cli(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.args(args);
    cmd
}

fn text(flag: &str, input: &str) -> String {
    let out = cli(&["-v", "ca", "-q", flag, "--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("apostrophe_not_a_symbol_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(input.as_bytes())));
    let status = cli(&["-D", "-v", "ca", "-w"]).arg(&path).arg(input).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

/// (input, `-x`, `--ipa`, samples, PCM hash)
type Case = (&'static str, &'static str, &'static str, usize, u64);

const CASES: [Case; 2] = [
    ("s' ha", "'es@ 'a", "ˈesə ˈa", 15106, 0x42b6_6a0d_83a1_6751),
    ("'hola", "'Ol@", "ˈɔlə", 11687, 0xe3c7_38fa_2f27_2ca9),
];

#[test]
fn a_stray_apostrophe_is_not_read_as_its_dictionary_name() {
    if !std::path::Path::new("espeak-ng-data/ca_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .filter_map(|&(input, x, ipa, samples, hash)| {
            let (got_x, got_ipa) = (text("-x", input), text("--ipa", input));
            let wav = pcm(input);
            (got_x != x || got_ipa != ipa || wav.len() / 2 != samples || fnv1a(&wav) != hash).then(|| {
                format!("{input:?}: -x {got_x:?} (C {x:?}), --ipa {got_ipa:?} (C {ipa:?}), {} samples (C {samples})", wav.len() / 2)
            })
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
