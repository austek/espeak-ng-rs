// A single letter of the language's `char_plus_apostrophe` set (Greek `σ`, Ido `l`) keeps its
// apostrophe, and `TranslateClause` skips the blank behind the apostrophe whatever follows it.
// A dot, bracket or quote then stands against the letter in one word, which is no longer
// spelled; a dash that stood between blanks is still a pause.
// Expected values come from `espeak-ng -v <voice> -q -x|--ipa <text>` and
// `espeak-ng -v <voice> -w out.wav <text>` (sample count and FNV-1a 64 hash of the PCM bytes),
// C f0fb11a8.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn cli(voice: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.args(["-v", voice]).args(args);
    cmd
}

fn text(voice: &str, markup: bool, flag: &str, input: &str) -> String {
    let out = cli(voice, &["-q", flag]).args(markup.then_some("-m")).args(["--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(voice: &str, markup: bool, input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("char_plus_apostrophe_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(format!("{voice}{input}").as_bytes())));
    let status = cli(voice, &["-w"]).arg(&path).args(markup.then_some("-m")).args(["--", input]).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

/// (voice, SSML, input, `-x`, `--ipa`, samples, PCM hash)
type Case = (&'static str, bool, &'static str, &'static str, &'static str, usize, u64);

const CASES: [Case; 6] = [
    ("el", false, "σ' . αυτό", "stel'i;a_:_: aft'o", "stelˈia aftˈo", 24164, 0x8f55_5c9d_3051_7ad2),
    ("el", false, "σ' (αυτό)", "s_:_: aft'o", "s aftˈo", 18960, 0xdd7e_5d44_dc83_048e),
    ("el", false, "σ' \"αυτό\"", "s_:_: aft'o", "s aftˈo", 18960, 0xdd7e_5d44_dc83_048e),
    ("io", false, "l' . homo", "l_:_: h'omo", "l hˈomo", 17180, 0x65e3_5748_52ea_c7eb),
    ("el", false, "σ' - αυτό", "s'iQma_:_: aft'o", "sˈiɣma aftˈo", 24778, 0xc2ad_6822_fdc0_baa4),
    ("el", true, "<speak>σ' . αυτό</speak>", "stel'i;a_:_: aft'o", "stelˈia aftˈo", 24318, 0xc746_ba57_9d14_b672),
];

#[test]
fn a_letter_and_its_apostrophe_take_what_follows_the_blank() {
    if !std::path::Path::new("espeak-ng-data/el_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .filter_map(|&(voice, markup, input, x, ipa, samples, hash)| {
            let (got_x, got_ipa) = (text(voice, markup, "-x", input), text(voice, markup, "--ipa", input));
            let wav = pcm(voice, markup, input);
            (got_x != x || got_ipa != ipa || wav.len() / 2 != samples || fnv1a(&wav) != hash).then(|| {
                format!("{voice} {input:?}: -x {got_x:?} (C {x:?}), --ipa {got_ipa:?} (C {ipa:?}), {} samples (C {samples})", wav.len() / 2)
            })
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
