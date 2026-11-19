// `TranslateClause` turns a dash between blanks (and a doubled one) into a blank with a pause
// before the next word.  The word list never sees it, so a language whose list names the
// hyphen (Greek `minus`) does not say that name.
// Expected values come from `espeak-ng -v el -q -x|--ipa <text>` and
// `espeak-ng -v el -w out.wav <text>` (sample count and FNV-1a 64 hash of the PCM bytes),
// C f0fb11a8.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn cli(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.args(["-v", "el"]).args(args);
    cmd
}

fn text(flag: &str, input: &str) -> String {
    let out = cli(&["-q", flag, "--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("spaced_dash_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(input.as_bytes())));
    let status = cli(&["-w"]).arg(&path).arg("--").arg(input).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

/// (input, `-x`, `--ipa`, samples, PCM hash)
type Case = (&'static str, &'static str, &'static str, usize, u64);

const CASES: [Case; 3] = [
    ("αβ - αυτό", "'av_:_: aft'o", "ˈav aftˈo", 20908, 0xe485_3fcb_08d5_02a3),
    ("αβ -- αυτό", "'av_:_: aft'o", "ˈav aftˈo", 20908, 0xe485_3fcb_08d5_02a3),
    ("αβ - - αυτό", "'av_:_: aft'o", "ˈav aftˈo", 20908, 0xe485_3fcb_08d5_02a3),
];

#[test]
fn a_dash_between_blanks_is_a_pause_in_greek() {
    if !std::path::Path::new("espeak-ng-data/el_dict").exists() {
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
