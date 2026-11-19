// `TranslateClause` starts a new word at a capital that follows a lower-case letter, except
// in Irish (`UpperCaseInWord`), where a capital after the eclipsis and lenition prefixes
// `bp bhf dt gc hA mb nd ng ts tA nA` stays in the word.  Every other capital splits, so
// `aBc` reads as `a` and `bc`.
// Expected values come from `espeak-ng -v ga -q -x|--ipa <text>` and
// `espeak-ng -v ga -w out.wav <text>` (sample count and FNV-1a 64 hash of the PCM bytes),
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
    let out = cli(&["-v", "ga", "-q", flag, "--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("irish_upper_case_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(input.as_bytes())));
    let status = cli(&["-D", "-v", "ga", "-w"]).arg(&path).arg(input).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

/// (input, `-x`, `--ipa`, samples, PCM hash)
type Case = (&'static str, &'static str, &'static str, usize, u64);

const CASES: [Case; 4] = [
    ("aBc", "@ b,i:s'i:", "ə bˌiːsˈiː", 19184, 0xcefe_8e7d_8454_e6f1),
    ("CamelCase", "k'0m@l; k'0S@", "kˈAməlʲ kˈAʃə", 23853, 0xaaa9_8b7c_5bfb_629d),
    ("bPlean", "bl;'an", "blʲˈan", 15054, 0x2197_e8fb_9e92_4c83),
    ("an tAthair", "@n t['ah@r", "ən t̪ˈahər", 17293, 0xdffc_a153_13b4_6c08),
];

#[test]
fn irish_capitals_split_words_like_c() {
    if !std::path::Path::new("espeak-ng-data/ga_dict").exists() {
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
