// An `<emphasis>` that sits between runs of dotted initials keeps its effect on the words
// after the joined runs: the emphasis level is the one the commands before each word leave.
// Expected values come from `espeak-ng -m -v <voice> -q -x|--ipa <text>` and
// `espeak-ng -m -v <voice> -w out.wav <text>` (PCM byte count and FNV-1a 64 hash), C f0fb11a8.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn cli(voice: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.args(["-m", "-v", voice]).args(args);
    cmd
}

fn text(voice: &str, flag: &str, input: &str) -> String {
    let out = cli(voice, &["-q", flag]).args(["--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(voice: &str, input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("dotted_emphasis_order_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(format!("{voice}{input}").as_bytes())));
    let status = cli(voice, &["-w"]).arg(&path).args(["--", input]).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

/// (voice, input, `-x`, `--ipa`, PCM bytes, PCM hash)
type Case = (&'static str, &'static str, &'static str, &'static str, usize, u64);

const CASES: [Case; 2] = [
    ("en", "<speak>one two <emphasis>u.s.a.'s</emphasis> e.g. x.y.</speak>", "w'0n t'u:_! j,u:,Es'eI_! z f,O@rEgz'aamp@L ,Eksw'aI", "wˈɒn tˈuː jˌuːˌɛsˈeɪ z fˌɔːɹɛɡzˈampəl ˌɛkswˈaɪ", 121440, 0x62c911d1e5d5c963),
    ("en", "<speak>a.b. c <emphasis>d.e. f</emphasis> g.h. i</speak>", ",eIb,i:s'i:_! d,i:;,i:;'Ef dZ,i:;,eItS'aI", "ˌeɪbˌiːsˈiː dˌiːˌiːˈɛf dʒˌiːˌeɪtʃˈaɪ", 78946, 0x243839eab2b5f542),
];

#[test]
fn emphasis_before_dotted_initials_reaches_the_words_after_them() {
    for (voice, input, x, ipa, bytes, hash) in CASES {
        assert_eq!(text(voice, "-x", input), x, "{voice} -x {input:?}");
        assert_eq!(text(voice, "--ipa", input), ipa, "{voice} --ipa {input:?}");
        let audio = pcm(voice, input);
        assert_eq!((audio.len(), fnv1a(&audio)), (bytes, hash), "{voice} wav {input:?}");
    }
}
