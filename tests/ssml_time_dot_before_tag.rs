// In SSML the reader keeps a full stop that a tag follows (`<speak>10:30.</speak>`) in the clause
// text, so the minutes are `30.` and no blank follows them: the time rule for the colon does not
// match and C reads the colon as a word.  The port dropped the dot and read a plain time.
// Expected values come from `espeak-ng -v en -m -q -x|--ipa <text>` and
// `espeak-ng -v en -m -w out.wav <text>` (sample count and FNV-1a 64 hash of the PCM bytes),
// C f0fb11a8.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn cli(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.args(["-v", "en", "-m"]).args(args);
    cmd
}

fn text(flag: &str, input: &str) -> String {
    let out = cli(&["-q", flag, "--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("ssml_time_dot_{}", std::process::id()));
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

const CASES: [Case; 3] = [
    ("<speak>10:30.</speak>", "t'En k'oUl@n T'3:ti", "tˈɛn kˈəʊlən θˈɜːti", 30968, 0x1b47_dff8_c22e_d914),
    ("<speak>x 10:30.</speak>", "'Eks t'En k'oUl@n T'3:ti", "ˈɛks tˈɛn kˈəʊlən θˈɜːti", 35984, 0x5e37_e5b3_e03a_40f4),
    ("<speak>3:2.</speak>", "Tr'i: k'oUl@n t'u:", "θɹˈiː kˈəʊlən tˈuː", 27636, 0x7d17_14ad_2415_29e1),
];

#[test]
fn a_dot_before_a_tag_keeps_the_time_a_ratio() {
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
