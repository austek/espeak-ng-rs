// A `$pause` word (`and`, `or`, `but`) is not the second word of a clause, so it takes its
// pause, when the words in front of it are dotted initials: C reads `a.b. x` as the words
// `a . b . x`, so the word before `and` is not the first one.  The pause was lost once
// the initials became one token.
// Expected values come from `espeak-ng -v en -q -x|--ipa <text>` and
// `espeak-ng -v en -w out.wav <text>` (sample count and FNV-1a 64 hash of the PCM bytes),
// C f0fb11a8.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn cli(markup: bool, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.args(["-v", "en"]).args(markup.then_some("-m")).args(args);
    cmd
}

fn text(markup: bool, flag: &str, input: &str) -> String {
    let out = cli(markup, &["-q", flag, "--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(markup: bool, input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("dotted_pause_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(input.as_bytes())));
    let status = cli(markup, &["-w"]).arg(&path).arg("--").arg(input).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

/// (SSML, input, `-x`, `--ipa`, samples, PCM hash)
type Case = (bool, &'static str, &'static str, &'static str, usize, u64);

const CASES: [Case; 4] = [
    (false, "a.b.c. x. and y", ",eIb,i:s,i:;'Eks_:_: and w'aI", "ˌeɪbˌiːsˌiːˈɛks and wˈaɪ", 36155, 0xa383_7bed_bcbd_fd2e),
    (false, "a.b. but y", ",eIb'i:_:_: b,Vt w'aI", "ˌeɪbˈiː bˌʌt wˈaɪ", 28532, 0x0b5e_4bb8_24c7_a1be),
    (false, "a.b. x or y", ",eIb,i:;'Eks_:_: O@ w'aI", "ˌeɪbˌiːˈɛks ɔː wˈaɪ", 31050, 0x3ddc_c946_5654_25d1),
    (
        true,
        "<speak>a.b.c. x and <emphasis>y</emphasis></speak>",
        ",eIb,i:s,i:;,Eks_:_: and_! w'aI",
        "ˌeɪbˌiːsˌiːˌɛks and wˈaɪ",
        37637,
        0x0755_e795_0570_117d,
    ),
];

#[test]
fn a_pause_word_after_dotted_initials_keeps_its_pause() {
    if !std::path::Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .filter_map(|&(markup, input, x, ipa, samples, hash)| {
            let (got_x, got_ipa) = (text(markup, "-x", input), text(markup, "--ipa", input));
            let wav = pcm(markup, input);
            (got_x != x || got_ipa != ipa || wav.len() / 2 != samples || fnv1a(&wav) != hash).then(|| {
                format!("{input:?}: -x {got_x:?} (C {x:?}), --ipa {got_ipa:?} (C {ipa:?}), {} samples (C {samples})", wav.len() / 2)
            })
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
