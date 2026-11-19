// `CheckDottedAbbrev` carries on with a one-letter word that follows the last dot of a run of
// initials: `a.b.c. x` is the four letters `abcx` read as one spelled word, so the last
// letter gets no stress of its own.  The port dropped the dot and spelled `abc` and `x`
// apart.
// Expected values come from `espeak-ng -v en -q -x|--ipa <text>` and
// `espeak-ng -v en -w out.wav <text>` (sample count and FNV-1a 64 hash of the PCM bytes),
// C f0fb11a8.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn cli(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.args(["-v", "en"]).args(args);
    cmd
}

fn text(flag: &str, input: &str) -> String {
    let out = cli(&["-q", flag, "--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("dotted_initials_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(input.as_bytes())));
    let status = cli(&["-D", "-w"]).arg(&path).arg(input).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

/// (input, `-x`, `--ipa`, samples, PCM hash)
type Case = (&'static str, &'static str, &'static str, usize, u64);

const CASES: [Case; 5] = [
    ("a.b.c. x", ",eIb,i:s,i:;'Eks", "ˌeɪbˌiːsˌiːˈɛks", 22752, 0x985a_3282_d840_4f10),
    ("a.b.c. y", ",eIb,i:s,i:w'aI", "ˌeɪbˌiːsˌiːwˈaɪ", 23391, 0xfc0a_ac27_f829_ba1e),
    ("u.s.a. x", "j,u:,Es,eI'Eks", "jˌuːˌɛsˌeɪˈɛks", 22051, 0x2b0f_e0d7_8b0f_14c3),
    ("i.e. x", ",aI,i:;'Eks", "ˌaɪˌiːˈɛks", 17976, 0xfab6_dfa6_dabf_9764),
    ("a.b.c. x y", ",eIb,i:s,i:;'Eks w'aI", "ˌeɪbˌiːsˌiːˈɛks wˈaɪ", 28780, 0x34cd_f608_ec22_992d),
];

#[test]
fn a_letter_after_the_last_dot_joins_the_initials() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
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
