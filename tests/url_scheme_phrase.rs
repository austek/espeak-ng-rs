// `TranslateClause` puts the `://` behind `http` in one word of symbols, so the list entry
// `(http ://)` that most languages carry matches and reads the scheme as a unit.  The port
// used to speak the colon and the two slashes as words of their own.
// Expected values come from `espeak-ng -v <lang> [-m] -q -x|--ipa <text>` and
// `espeak-ng -v <lang> [-m] -w out.wav <text>` (sample count and FNV-1a 64 hash of the PCM
// bytes), C f0fb11a8.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn cli(lang: &str, ssml: bool, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.args(["-v", lang]);
    if ssml {
        cmd.arg("-m");
    }
    cmd.args(args);
    cmd
}

fn text(lang: &str, ssml: bool, flag: &str, input: &str) -> String {
    let out = cli(lang, ssml, &["-q", flag, "--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(lang: &str, ssml: bool, input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("url_scheme_phrase_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(format!("{lang}{ssml}{input}").as_bytes())));
    let status = cli(lang, ssml, &["-D", "-w"]).arg(&path).arg(input).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

/// (language, SSML, input, `-x`, `--ipa`, samples, PCM hash)
type Case = (&'static str, bool, &'static str, &'static str, &'static str, usize, u64);

const CASES: [Case; 8] = [
    ("en", false, "see http://example.com/a now", "s'i:; ,eItSti:ti:p'i:_ Egz'aamp@L d'0t k'0m sl'aS a# n'aU", "sˈiː ˌeɪtʃtiːtiːpˈiː ɛɡzˈampəl dˈɒt kˈɒm slˈaʃ ɐ nˈaʊ", 66310, 0x2591_911f_629f_9f6f),
    ("en", false, "http://x", ",eItSti:ti:p'i:_ 'Eks", "ˌeɪtʃtiːtiːpˈiː ˈɛks", 30103, 0x7ead_daaf_4295_2f92),
    ("fr", false, "http://x", "aSt,et,ep'e_ _!'iks", "aʃtˌetˌepˈe ˈiks", 26183, 0xb969_790e_1221_9b04),
    ("af", false, "http://x", "h,A:te@te@p'e@_ _!'Eks", "hˌɑːtiətiəpˈiə ˈɛks", 31556, 0x2a61_70da_a5c1_3b95),
    ("cs", false, "http://x", "h'a:tet,epe: 'iks", "hˈaːtetˌepeː ˈiks", 26113, 0xc769_c729_a427_2215),
    ("bg", false, "http://x", ",ejtSti:ti:p'i:_ (en)'Eks(bg)", "ˌejtʃtiːtiːpˈiː (en)ˈɛks(bg)", 30992, 0xb465_0260_4734_7fc4),
    ("ka", false, "http://x", "'ejtStit,iphi_ (en)'Eks(ka)", "ˈejtʃtitˌiphi (en)ˈɛks(ka)", 30745, 0xd29e_6daa_ca8f_c63a),
    ("en", true, "<speak>see http://example.com now</speak>", "s'i:; ,eItSti:ti:p'i:_ Egz'aamp@L d'0t k'0m n'aU", "sˈiː ˌeɪtʃtiːtiːpˈiː ɛɡzˈampəl dˈɒt kˈɒm nˈaʊ", 57658, 0x8a25_f928_c7b9_99d3),
];

#[test]
fn the_url_scheme_matches_its_list_entry_like_c() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .filter_map(|&(lang, ssml, input, x, ipa, samples, hash)| {
            let (got_x, got_ipa) = (text(lang, ssml, "-x", input), text(lang, ssml, "--ipa", input));
            let wav = pcm(lang, ssml, input);
            (got_x != x || got_ipa != ipa || wav.len() / 2 != samples || fnv1a(&wav) != hash).then(|| {
                format!("{lang} {input:?}: -x {got_x:?} (C {x:?}), --ipa {got_ipa:?} (C {ipa:?}), {} samples (C {samples})", wav.len() / 2)
            })
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
