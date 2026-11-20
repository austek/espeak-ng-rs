// A word that ends in an apostrophe with a mark glued behind it still goes through the
// unpronounceable test: `Unpronouncable` takes only a single letter in front of the apostrophe
// (`l'`) for pronounceable, so `bk'(` is spelled letter by letter.
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

fn text(voice: &str, flag: &str, input: &str) -> String {
    let out = cli(voice, &["-q", flag]).args(["--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(voice: &str, input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("apostrophe_unpronounceable_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(format!("{voice}{input}").as_bytes())));
    let status = cli(voice, &["-w"]).arg(&path).args(["--", input]).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

/// (voice, input, `-x`, `--ipa`, samples, PCM hash)
type Case = (&'static str, &'static str, &'static str, &'static str, usize, u64);

const CASES: [Case; 3] = [
    ("eo", "bk'(", "b,ok'o", "bˌokˈo", 14103, 0x4bd3fcf0951d8240),
    ("eo", "Dr'<  ab", "d,oR'o_:_: 'ab", "dˌorˈo ˈab", 20407, 0x927c5b88763c4bf6),
    ("eo", "bk’<", "b,ok'o", "bˌokˈo", 14103, 0x4bd3fcf0951d8240),
];

#[test]
fn a_word_of_two_letters_and_an_apostrophe_is_spelled() {
    for (voice, input, x, ipa, samples, hash) in CASES {
        assert_eq!(text(voice, "-x", input), x, "{voice} -x {input:?}");
        assert_eq!(text(voice, "--ipa", input), ipa, "{voice} --ipa {input:?}");
        let audio = pcm(voice, input);
        assert_eq!((audio.len() / 2, fnv1a(&audio)), (samples, hash), "{voice} wav {input:?}");
    }
}
