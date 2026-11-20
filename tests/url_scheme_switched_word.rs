// A Latin word that a voice of another script hands to English is translated in English together
// with the words behind it, so the list entry `(http ://)` swallows the `://` of a URL there as
// it does in an English voice; in the voice's own reading the symbols would be spoken.
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
    let dir = std::env::temp_dir().join(format!("url_scheme_switched_word_{}", std::process::id()));
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

const CASES: [Case; 12] = [
    ("ko", "http://x", "(en),eItSti:ti:p'i:_ 'Eks(ko)", "(en)ˌeɪtʃtiːtiːpˈiː ˈɛks(ko)", 30484, 0x0c7c_eca0_e3b3_994a),
    ("el", "http://x", "(en),eItSti:ti:p'i:_ 'Eks(el)", "(en)ˌeɪtʃtiːtiːpˈiː ˈɛks(el)", 30201, 0xd3b0_d89f_ca2c_7fce),
    ("ar", "http://x", "(en),eItSti:ti:p'i:_ 'Eks(ar)", "(en)ˌeɪtʃtiːtiːpˈiː ˈɛks(ar)", 30600, 0x26db_433f_68db_0f4f),
    ("he", "http://x", "(en),eItSti:ti:p'i:_ 'Eks(he)", "(en)ˌeɪtʃtiːtiːpˈiː ˈɛks(he)", 30600, 0x26db_433f_68db_0f4f),
    ("hi", "http://x", "(en),eItSti:ti:p'i:_ 'Eks(hi)", "(en)ˌeɪtʃtiːtiːpˈiː ˈɛks(hi)", 29792, 0x5065_ecda_30c6_356c),
    ("th", "http://x", "(en),eItSti:ti:p'i:_ 'Eks(th)", "(en)ˌeɪtʃtiːtiːpˈiː ˈɛks(th)", 30600, 0x26db_433f_68db_0f4f),
    ("yue", "http://x", "(en),eI1tSti:1ti:1p'i:1__| 'E1ks(yue)_|", "(en)ˌeɪ1tʃtiː1tiː1pˈiː1 ˈɛ1ks(yue)", 29618, 0xa810_62f7_dc09_7b37),
    ("ru", "http://x", "(en),eItSti:ti:p'i:_ 'Eks(ru)", "(en)ˌeɪtʃtiːtiːpˈiː ˈɛks(ru)", 30600, 0x59a0_0f1d_e6f8_3fbc),
    ("ko", "abc://x", "(en),eIb,i:s'i: k'oUl@nsl,aSslaS 'Eks(ko)", "(en)ˌeɪbˌiːsˈiː kˈəʊlənslˌaʃslaʃ ˈɛks(ko)", 44366, 0x4496_e677_d611_a5d6),
    ("ko", "http://", "(en),eItSti:ti:p'i:_(ko)", "(en)ˌeɪtʃtiːtiːpˈiː(ko)", 24708, 0x48de_067c_bc0e_7a38),
    ("ko", "http ://x", "(en),eItSti:ti:p'i:_ 'Eks(ko)", "(en)ˌeɪtʃtiːtiːpˈiː ˈɛks(ko)", 30484, 0x0c7c_eca0_e3b3_994a),
    ("ko", "http:// x", "(en),eItSti:ti:p'i:_ 'Eks(ko)", "(en)ˌeɪtʃtiːtiːpˈiː ˈɛks(ko)", 30484, 0x0c7c_eca0_e3b3_994a),
];

/// Mandarin's English letters are voiced differently from C (a separate gap), so only the phonemes
/// are compared.
#[test]
fn the_scheme_of_a_url_has_the_phonemes_of_c_in_mandarin() {
    assert_eq!(text("cmn", "-x", "http://x"), "(en),eI55tSti:55ti:55p'i:55__| 'E55ks(cmn)_|");
    assert_eq!(text("cmn", "--ipa", "http://x"), "(en)ˌeɪ5tʃtiː5tiː5pˈiː5 ˈɛ5ks(cmn)");
}

#[test]
fn the_scheme_of_a_url_is_read_by_the_english_entry() {
    for (voice, input, x, ipa, samples, hash) in CASES {
        assert_eq!(text(voice, "-x", input), x, "{voice} -x {input:?}");
        assert_eq!(text(voice, "--ipa", input), ipa, "{voice} --ipa {input:?}");
        let audio = pcm(voice, input);
        assert_eq!((audio.len() / 2, fnv1a(&audio)), (samples, hash), "{voice} wav {input:?}");
    }
}
