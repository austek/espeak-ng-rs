// C ends a word at a symbol that follows a letter (`a@b` is `a @b` in the clause buffer), so a
// single letter glued to a symbol still joins the dotted initials before it: `e.g. a@b` spells
// `e.g.a` and takes no `$pause` of its own.
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
    let dir = std::env::temp_dir().join(format!("dotted_initials_symbol_{}", std::process::id()));
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

const CASES: [Case; 11] = [
    ("sk", "e.g. a@b", "'e:g,e:,a: z'avina:dZ b'e:", "ˈeːɡˌeːˌaː zˈavinaːdʒ bˈeː", 36280, 0x2b42_2ea0_6aa0_be74),
    ("sk", "x.y. a@b", "'iks,ipsilon,a: z'avina:dZ b'e:", "ˈiksˌipsilonˌaː zˈavinaːdʒ bˈeː", 41276, 0x049d_801a_e0af_7bae),
    ("sk", "i.e. a#b", "'i:;,e:,a: kR'i:Zik b'e:", "ˈiːˌeːˌaː krˈiːʒik bˈeː", 32405, 0x7a61_624d_3b20_ca77),
    ("sk", "e.g. a/b", "'e:g,e:,a: b'e:", "ˈeːɡˌeːˌaː bˈeː", 24072, 0x7ba8_eca7_becb_30ed),
    ("sk", "e.g. a ty", "'e:g,e:,a: t'i", "ˈeːɡˌeːˌaː tˈi", 20342, 0x6ced_74e4_2b90_0c1a),
    ("sk", "e.g. a@ b", "'e:g,e:,a: z'avina:dZ b'e:", "ˈeːɡˌeːˌaː zˈavinaːdʒ bˈeː", 36280, 0x2b42_2ea0_6aa0_be74),
    ("cs", "e.g. a@b", "'e:g,e:,a: z'avina:dZ b'e:", "ˈeːɡˌeːˌaː zˈavinaːdʒ bˈeː", 35847, 0x12a2_e312_adc0_ea70),
    ("hr", "e.g. a@b", "'Eg,@,& _'ad b'@", "ˈɛɡˌəˌæ ˈad bˈə", 21429, 0x49d0_836a_9072_8814),
    ("cy", "e.g. a@b", ",E,Eg'A: (en)_at_(cy) b'i:", "ˌɛˌɛɡˈɑː (en)at(cy) bˈiː", 26400, 0x3400_92a0_1204_e8e6),
    ("en", "e.g. a@b", "f,O@rEgz'aamp@L a# _at_ b'i:", "fˌɔːɹɛɡzˈampəl ɐ at bˈiː", 36251, 0x55c2_9737_1380_01f2),
    ("sk", "e.g. ab@b", "'e:g,e: 'ab z'avina:dZ b'e:", "ˈeːɡˌeː ˈab zˈavinaːdʒ bˈeː", 36269, 0xf343_2c85_d62f_9217),
];

#[test]
fn a_letter_glued_to_a_symbol_joins_the_dotted_initials() {
    for (voice, input, x, ipa, samples, hash) in CASES {
        assert_eq!(text(voice, "-x", input), x, "{voice} -x {input:?}");
        assert_eq!(text(voice, "--ipa", input), ipa, "{voice} --ipa {input:?}");
        let audio = pcm(voice, input);
        assert_eq!((audio.len() / 2, fnv1a(&audio)), (samples, hash), "{voice} wav {input:?}");
    }
}
