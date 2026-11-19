// `TranslateClause` keeps a single letter of `char_plus_apostrophe` and its apostrophe as a
// word of their own: Greek `σ' αυτό` joins into the word `σ'αυτό` (the blank after the
// apostrophe is skipped), Ido `l' homo` likewise, while Esperanto's voice file sets
// `apostrophe 2`, which keeps the apostrophe at the end of the word and the blank in place.
// Expected values come from `espeak-ng -v <lang> [-m] -q -x|--ipa <text>` and
// `espeak-ng -v <lang> -w out.wav <text>` (sample count and FNV-1a 64 hash of the PCM bytes),
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

fn text(lang: &str, flag: &str, input: &str) -> String {
    let out = cli(&["-v", lang, "-q", flag, "--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(lang: &str, input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("char_plus_apostrophe_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(format!("{lang}{input}").as_bytes())));
    let status = cli(&["-D", "-v", lang, "-w"]).arg(&path).arg(input).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

/// (language, input, `-x`, `--ipa`, samples, PCM hash)
type Case = (&'static str, &'static str, &'static str, &'static str, usize, u64);

const CASES: [Case; 5] = [
    ("el", "σ' αυτό", "saft'o", "saftˈo", 15887, 0xa1e4_7369_32ee_7415),
    ("el", "πάω σ' εσένα.", "p'ao ses'ena", "pˈao sesˈena", 22146, 0xb0fc_a592_6b59_103f),
    ("io", "l' homo", "lh'omo", "lhˈomo", 14492, 0x92d9_b4cd_8a39_68ea),
    ("eo", "l' homo", "l'a h'omo", "lˈa hˈomo", 16525, 0xf9d5_f678_0dfa_f02f),
    ("eo", "mi vidas l' hundon.", "mi v'idas l'a h'undon", "mi vˈidas lˈa hˈundon", 29680, 0x0f6a_01c0_3c6a_bd6c),
];

#[test]
fn single_letter_and_apostrophe_cut_like_c() {
    if !std::path::Path::new("espeak-ng-data/el_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .filter_map(|&(lang, input, x, ipa, samples, hash)| {
            let (got_x, got_ipa) = (text(lang, "-x", input), text(lang, "--ipa", input));
            let wav = pcm(lang, input);
            (got_x != x || got_ipa != ipa || wav.len() / 2 != samples || fnv1a(&wav) != hash).then(|| {
                format!("{lang} {input:?}: -x {got_x:?} (C {x:?}), --ipa {got_ipa:?} (C {ipa:?}), {} samples (C {samples})", wav.len() / 2)
            })
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
