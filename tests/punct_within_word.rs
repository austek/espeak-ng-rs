// `punct_within_word` lists the characters `TranslateClause` leaves inside a word: Catalan
// keeps the middle dot (`col·legi` is one word, not `col` and `legi`), Lojban the comma
// (`lo,e` and `coi,alis` are one word each).
// Expected values come from `espeak-ng -v <lang> -q -x|--ipa <text>` and
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
    let dir = std::env::temp_dir().join(format!("punct_within_word_{}", std::process::id()));
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

const CASES: [Case; 3] = [
    ("ca", "col·legi", "kull'EZi", "kullˈɛʒi", 17952, 0x6b78_45e1_9e82_99ae),
    ("jbo", "lo,e", "l'oe", "lˈoe", 19101, 0x28b1_7b69_5318_e2ab),
    ("jbo", "coi,alis", "SoI'alis_!", "ʃoɪˈalis", 26174, 0x0555_db4f_c047_9756),
];

#[test]
fn punctuation_within_a_word_stays_in_the_word() {
    if !std::path::Path::new("espeak-ng-data/ca_dict").exists() {
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
