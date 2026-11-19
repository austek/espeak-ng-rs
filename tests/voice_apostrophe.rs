// A voice file's `apostrophe <bits>` sets `LOPT_APOSTROPHE` for the language it names: bit 0
// keeps an apostrophe in front of a letter in the word, bit 1 one behind a letter.
// The voices are two files added to a copy of the data directory (symbolic links to the
// shipped data); expected values come from `espeak-ng -v <voice> -q -x|--ipa <text>` and
// `espeak-ng -v <voice> -w out.wav <text>` (sample count and FNV-1a 64 hash of the PCM bytes)
// on C f0fb11a8 with the same two files.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

/// A data directory with the shipped data and the voices `apos1` and `apos2`.
fn data_with_voices() -> PathBuf {
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("espeak-ng-data");
    let dir = std::env::temp_dir().join(format!("voice_apostrophe_{}", std::process::id()));
    let voices = dir.join("voices");
    std::fs::create_dir_all(&voices).expect("voices dir");
    for entry in std::fs::read_dir(&shipped).expect("data dir").flatten() {
        let name = entry.file_name();
        if name == "voices" {
            for voice in std::fs::read_dir(entry.path()).expect("voices").flatten() {
                let _ = std::os::unix::fs::symlink(voice.path(), voices.join(voice.file_name()));
            }
        } else {
            let _ = std::os::unix::fs::symlink(entry.path(), dir.join(&name));
        }
    }
    for (name, bits) in [("apos1", 1), ("apos2", 2)] {
        std::fs::write(voices.join(name), format!("name {name}\nlanguage en\napostrophe {bits}\n")).expect("voice file");
    }
    dir
}

fn cli(data: &Path, voice: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.env("ESPEAK_DATA_PATH", data).args(["-v", voice]).args(args);
    cmd
}

fn text(data: &Path, voice: &str, flag: &str, input: &str) -> String {
    let out = cli(data, voice, &["-q", flag, "--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(data: &Path, voice: &str, input: &str) -> Vec<u8> {
    let path = data.join(format!("{:x}.wav", fnv1a(format!("{voice}{input}").as_bytes())));
    let status = cli(data, voice, &["-w"]).arg(&path).arg("--").arg(input).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    bytes[44..].to_vec()
}

/// (voice, input, `-x`, `--ipa`, samples, PCM hash)
type Case = (&'static str, &'static str, &'static str, &'static str, usize, u64);

const CASES: [Case; 4] = [
    ("apos2", "the boys' toys", "D@ b'OIs t'OIz", "ðə bˈɔɪs tˈɔɪz", 25165, 0xda9f_fbb8_eced_9683),
    ("apos2", "rock' roll", "r'0k r'oUl", "ɹˈɒk ɹˈəʊl", 18263, 0x67e1_f80b_c8d3_df3d),
    ("apos1", "rock' roll", "r'0k_:_: r'oUl", "ɹˈɒk ɹˈəʊl", 21974, 0x883e_bdee_b1fb_dc00),
    ("apos1", "'tis the 'boys' toy", "t'aIz D@ b'OIz t'OI", "tˈaɪz ðə bˈɔɪz tˈɔɪ", 31058, 0x3419_7570_b024_5823),
];

#[test]
fn the_voice_file_sets_the_apostrophe_bits() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let data = data_with_voices();
    let bad: Vec<String> = CASES
        .iter()
        .filter_map(|&(voice, input, x, ipa, samples, hash)| {
            let (got_x, got_ipa) = (text(&data, voice, "-x", input), text(&data, voice, "--ipa", input));
            let wav = pcm(&data, voice, input);
            (got_x != x || got_ipa != ipa || wav.len() / 2 != samples || fnv1a(&wav) != hash).then(|| {
                format!("{voice} {input:?}: -x {got_x:?} (C {x:?}), --ipa {got_ipa:?} (C {ipa:?}), {} samples (C {samples})", wav.len() / 2)
            })
        })
        .collect();
    let _ = std::fs::remove_dir_all(&data);
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
