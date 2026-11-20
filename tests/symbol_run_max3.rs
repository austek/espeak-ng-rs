// Symbols that stand together are one word whose characters C reads one by one, and a character
// whose list entry carries `$max3` is dropped after its third repeat in a row
// (`phonemes_repeat_count`), inside a run as between blanks.
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
    let dir = std::env::temp_dir().join(format!("symbol_run_max3_{}", std::process::id()));
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

const CASES: [Case; 6] = [
    ("en", "a //// b", "a# sl'aSsla#Ssl,aS b'i:", "ɐ slˈaʃslɐʃslˌaʃ bˈiː", 33472, 0x3393588675d5b57e),
    ("en", "a ##### b", "a# h'aSha#Sh,aS b'i:", "ɐ hˈaʃhɐʃhˌaʃ bˈiː", 30772, 0xa2144eb395547ab1),
    ("de", "a ##### b", "_!'A: n'Um3n,Um3n,Um3 b'e:", "ˈɑː nˈʊmɜnˌʊmɜnˌʊmɜ bˈeː", 30117, 0x87cdc2962801fc84),
    ("en", "a ////////////////////////////////////////  b", "a# sl'aSsla#Ssl,aS b'i:", "ɐ slˈaʃslɐʃslˌaʃ bˈiː", 33472, 0x3393588675d5b57e),
    (
        "en",
        "a //// b //// c",
        "a# sl'aSsla#Ssl,aS b'i: sl'aSsla#Ssl,aS s'i:",
        "ɐ slˈaʃslɐʃslˌaʃ bˈiː slˈaʃslɐʃslˌaʃ sˈiː",
        56073,
        0xd8ec_abee_eb10_224d,
    ),
    (
        "en",
        "a /#/#/#/# b",
        "a# sl'aSha#Ssl,aSha#Ssl,aSha#Ssl,aShaS b'i:",
        "ɐ slˈaʃhɐʃslˌaʃhɐʃslˌaʃhɐʃslˌaʃhaʃ bˈiː",
        58292,
        0x8c60_e8f0_5127_b5ba,
    ),
];

#[test]
fn a_symbol_repeated_in_one_word_is_read_three_times() {
    for (voice, input, x, ipa, samples, hash) in CASES {
        assert_eq!(text(voice, "-x", input), x, "{voice} -x {input:?}");
        assert_eq!(text(voice, "--ipa", input), ipa, "{voice} --ipa {input:?}");
        let audio = pcm(voice, input);
        assert_eq!((audio.len() / 2, fnv1a(&audio)), (samples, hash), "{voice} wav {input:?}");
    }
}
