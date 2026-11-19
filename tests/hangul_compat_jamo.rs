// `TranslateChar` maps a Hangul compatibility jamo (`ㅅ`, U+3145) to the jamo of the same sound
// in U+1100.., which the Korean rules read.  The port left it as it was, found no rule for
// it and spoke the language name of the script instead, once per jamo.
// Expected values come from `espeak-ng -v ko -q -x|--ipa <text>` and
// `espeak-ng -v ko -w out.wav <text>` (sample count and FNV-1a 64 hash of the PCM bytes),
// C f0fb11a8.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn cli(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.args(["-v", "ko"]).args(args);
    cmd
}

fn text(flag: &str, input: &str) -> String {
    let out = cli(&["-q", flag, "--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("hangul_compat_jamo_{}", std::process::id()));
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
    ("ㅅ", "si;'ot-", "siˈot-", 15709, 0x1213_34c5_afea_365b),
    ("ㅅㅅ", "ss", "ss", 10756, 0xdbcc_37d9_74fb_3c54),
    ("가ㅅ", "g'as", "ɡˈɐs", 13857, 0x23a8_fb2d_f3ad_4f2f),
    ("ㄱ", "gij'@q", "ɡijˈʌq", 15740, 0x0532_7b9c_8f07_cdf8),
    ("가ㅏ", "g'aa", "ɡˈɐɐ", 13717, 0xfde8_3583_383a_428d),
];

#[test]
fn compatibility_jamo_are_read_as_jamo() {
    if !std::path::Path::new("espeak-ng-data/ko_dict").exists() {
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
