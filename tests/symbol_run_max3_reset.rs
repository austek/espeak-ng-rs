// The `$max3` repeat count starts again at every word that is a run of symbols (the lookup of
// the whole run clears it) and at every clause (`phonemes_repeat_count`).
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
    let dir = std::env::temp_dir().join(format!("symbol_run_max3_reset_{}", std::process::id()));
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

const CASES: [Case; 12] = [
    ("en", "## ## ##", "h'aShaS h'aShaS h'aShaS", "hˈaʃhaʃ hˈaʃhaʃ hˈaʃhaʃ", 81708, 0xe02113896422fe6c),
    ("en", "## ## ## ##", "h'aShaS h'aShaS h'aShaS h'aShaS", "hˈaʃhaʃ hˈaʃhaʃ hˈaʃhaʃ hˈaʃhaʃ", 102960, 0x57579751aec93d07),
    ("en", "a ##. ## b", "a# h'aShaS\nh'aShaS b'i:", "ɐ hˈaʃhaʃ\nhˈaʃhaʃ bˈiː", 89296, 0xf172a312688ad974),
    ("en", "a ##. ## ## b", "a# h'aShaS\nh'aShaS h'aShaS b'i:", "ɐ hˈaʃhaʃ\nhˈaʃhaʃ hˈaʃhaʃ bˈiː", 111232, 0x2ee7167413d6652e),
    ("fr", "a ++ x ++ ++ b", "a plyspl'ys _!'iks plyspl'ys plyspl'ys b'e", "a plysplˈys ˈiks plysplˈys plysplˈys bˈe", 101666, 0x7ae067aef04f3c96),
    (
        "en",
        "==== ====",
        "_:'i:kw@Lz_:_:,i:kw@Lz_:_:,i:kw@Lz_: _:'i:kw@Lz_:_:,i:kw@Lz_:_:,i:kw@Lz_:",
        "ˈiːkwəlzˌiːkwəlzˌiːkwəlz ˈiːkwəlzˌiːkwəlzˌiːkwəlz",
        155444,
        0x0aac_b4ba_485b_ad9d,
    ),
    ("en", "## # ## #", "h'aShaS h'aS h'aShaS h'aS", "hˈaʃhaʃ hˈaʃ hˈaʃhaʃ hˈaʃ", 84162, 0x2be212d02bbe3af3),
    ("en", "a ++ ++ b", "a# pl'Vspl'Vs pl'Vspl'Vs b'i:", "ɐ plˈʌsplˈʌs plˈʌsplˈʌs bˈiː", 73732, 0xd780638fb14db73f),
    ("en", "a // // b", "a# sl'aSslaS sl'aSslaS b'i:", "ɐ slˈaʃslaʃ slˈaʃslaʃ bˈiː", 80086, 0x8eac2625c8cab283),
    (
        "en",
        "a ~ ~ ~ ~. ~ ~ ~ ~ b",
        "a# t'Ild@ t'Ild@ t'Ild@\nt'Ild@ t'Ild@ t'Ild@ b'i:",
        "ɐ tˈɪldə tˈɪldə tˈɪldə\ntˈɪldə tˈɪldə tˈɪldə bˈiː",
        137614,
        0x5c78_f851_8482_8799,
    ),
    (
        "en",
        "a ~ ~ ~ ~ ? ~ ~ ~ ~ b",
        "a# t'Ild@ t'Ild@ t'Ild@\nt'Ild@ t'Ild@ t'Ild@ b'i:",
        "ɐ tˈɪldə tˈɪldə tˈɪldə\ntˈɪldə tˈɪldə tˈɪldə bˈiː",
        137606,
        0x0770_fcaa_4c13_fe7f,
    ),
    (
        "en",
        "a ~~~~ ~~~~ b",
        "a# t'Ild@t,Ild@t,Ild@ t'Ild@t,Ild@t,Ild@ b'i:",
        "ɐ tˈɪldətˌɪldətˌɪldə tˈɪldətˌɪldətˌɪldə bˈiː",
        113816,
        0x6d74_3539_7313_161a,
    ),
];

#[test]
fn symbol_runs_and_clauses_start_the_repeat_count_again() {
    for (voice, input, x, ipa, samples, hash) in CASES {
        assert_eq!(text(voice, "-x", input), x, "{voice} -x {input:?}");
        assert_eq!(text(voice, "--ipa", input), ipa, "{voice} --ipa {input:?}");
        let pcm = pcm(voice, input);
        assert_eq!((pcm.len(), fnv1a(&pcm)), (samples, hash), "{voice} wav {input:?}");
    }
}
