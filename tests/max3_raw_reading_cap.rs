// C compares a symbol's raw dictionary string (`ph_out` in `LookupWord`) with `phonemes_repeat`,
// a `char[20]` that keeps the first 19 bytes of the last string.  A raw entry of 19 bytes or
// fewer repeats at most three times in a row however long its spoken form grows once stress is
// added; a raw entry of 20 bytes or more never equals its stored copy.  Expected values come from
// `espeak-ng -v <voice> -q -x|--ipa <text>`, `-w out.wav` (PCM sample bytes and FNV-1a 64 hash)
// and the C library's word and end events, C f0fb11a8.

use std::path::Path;
use std::process::Command;

use espeak_ng::{EspeakNg, EventKind};

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
    let dir = std::env::temp_dir().join(format!("max3_raw_reading_cap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(format!("{voice}{input}").as_bytes())));
    let status = cli(voice, &["-w"]).arg(&path).args(["--", input]).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

/// Word and end events as `type:text position:length:number`, then the sample total.
fn events(voice: &str, input: &str) -> Option<String> {
    let dir = Path::new("espeak-ng-data");
    if !dir.join("phontab").exists() {
        return None;
    }
    let e = EspeakNg::builder().voice(voice).data_dir(dir).build().ok()?;
    let (pcm, _rate, events) = e.synth_with_events(input).ok()?;
    let mut parts: Vec<String> = events
        .iter()
        .filter_map(|ev| match ev.kind {
            EventKind::Word(n) => Some(format!("1:{}:{}:num={n}", ev.text_position, ev.length)),
            EventKind::End(n) => Some(format!("5:{}:{}:num={n}", ev.text_position, ev.length)),
            _ => None,
        })
        .collect();
    parts.push(format!("samples={}", pcm.len()));
    Some(parts.join(" "))
}

/// (voice, input, `-x`, `--ipa`)
type TextCase = (&'static str, &'static str, &'static str, &'static str);

const TEXT_CASES: &[TextCase] = &[
    ("fr", "¬ ¬ ¬ ¬ ¬", "tirE_fakyltat'if tirE_fakyltat'if tirE_fakyltat'if", "tiʁɛfakyltatˈif tiʁɛfakyltatˈif tiʁɛfakyltatˈif"),
    ("fr", "¬ ¬ ¬ ¬", "tirE_fakyltat'if tirE_fakyltat'if tirE_fakyltat'if", "tiʁɛfakyltatˈif tiʁɛfakyltatˈif tiʁɛfakyltatˈif"),
    ("uk", ", , , , ,", "k'oma k'oma k'oma", "kˈoma kˈoma kˈoma"),
    ("uk", ", , , ,", "k'oma k'oma k'oma", "kˈoma kˈoma kˈoma"),
    ("hi", "। । । । ।", "p'u:rnwI*'a:m p'u:rnwI*'a:m p'u:rnwI*'a:m", "pˈuːrnwɪɾˈaːm pˈuːrnwɪɾˈaːm pˈuːrnwɪɾˈaːm"),
    ("hi", "। । । ।", "p'u:rnwI*'a:m p'u:rnwI*'a:m p'u:rnwI*'a:m", "pˈuːrnwɪɾˈaːm pˈuːrnwɪɾˈaːm pˈuːrnwɪɾˈaːm"),
    ("hi", "॥ ॥ ॥ ॥ ॥", "d'i:*g@wI*'a:m d'i:*g@wI*'a:m d'i:*g@wI*'a:m", "dˈiːɾɡəwɪɾˈaːm dˈiːɾɡəwɪɾˈaːm dˈiːɾɡəwɪɾˈaːm"),
    ("hi", "॥ ॥ ॥ ॥", "d'i:*g@wI*'a:m d'i:*g@wI*'a:m d'i:*g@wI*'a:m", "dˈiːɾɡəwɪɾˈaːm dˈiːɾɡəwɪɾˈaːm dˈiːɾɡəwɪɾˈaːm"),
    ("cs", "! ! ! ! !", "v'igR^itSn^i:k v'igR^itSn^i:k v'igR^itSn^i:k", "vˈiɡr̝itʃɲiːk vˈiɡr̝itʃɲiːk vˈiɡr̝itʃɲiːk"),
    ("cs", "! ! ! !", "v'igR^itSn^i:k v'igR^itSn^i:k v'igR^itSn^i:k", "vˈiɡr̝itʃɲiːk vˈiɡr̝itʃɲiːk vˈiɡr̝itʃɲiːk"),
    ("hr", ": : : : :", "dv'ototSk& dv'ototSk& dv'ototSk&", "dvˈototʃkæ dvˈototʃkæ dvˈototʃkæ"),
    ("hr", ": : : :", "dv'ototSk& dv'ototSk& dv'ototSk&", "dvˈototʃkæ dvˈototʃkæ dvˈototʃkæ"),
    ("hr", "! ! ! ! !", "'usklItSnIk 'usklItSnIk 'usklItSnIk", "ˈusklɪtʃnɪk ˈusklɪtʃnɪk ˈusklɪtʃnɪk"),
    ("hr", "! ! ! !", "'usklItSnIk 'usklItSnIk 'usklItSnIk", "ˈusklɪtʃnɪk ˈusklɪtʃnɪk ˈusklɪtʃnɪk"),
    ("sr", ": : : : :", "dv'ot&tSk& dv'ot&tSk& dv'ot&tSk&", "dvˈotɐtʃkɐ dvˈotɐtʃkɐ dvˈotɐtʃkɐ"),
    ("sr", ": : : :", "dv'ot&tSk& dv'ot&tSk& dv'ot&tSk&", "dvˈotɐtʃkɐ dvˈotɐtʃkɐ dvˈotɐtʃkɐ"),
    ("sr", "! ! ! ! !", "'uzvItSnIk 'uzvItSnIk 'uzvItSnIk", "ˈuzvɪtʃnɪk ˈuzvɪtʃnɪk ˈuzvɪtʃnɪk"),
    ("sr", "! ! ! !", "'uzvItSnIk 'uzvItSnIk 'uzvItSnIk", "ˈuzvɪtʃnɪk ˈuzvɪtʃnɪk ˈuzvɪtʃnɪk"),
];

/// (voice, input, PCM bytes, FNV-1a 64 of the PCM bytes)
type WavCase = (&'static str, &'static str, usize, u64);

const WAV_CASES: &[WavCase] = &[
    ("fr", "¬ ¬ ¬ ¬ ¬", 142252, 0xa5f5c25089fc274e),
    ("fr", "¬ ¬ ¬ ¬", 142252, 0xa5f5c25089fc274e),
    ("uk", ", , , , ,", 71072, 0xdef546e8b4d5c918),
    ("uk", ", , , ,", 71072, 0xdef546e8b4d5c918),
    ("hi", "। । । । ।", 94644, 0x13bbfd1c09b929f7),
    ("hi", "। । । ।", 94644, 0x13bbfd1c09b929f7),
    ("hi", "॥ ॥ ॥ ॥ ॥", 107160, 0x80df43a532ce3782),
    ("hi", "॥ ॥ ॥ ॥", 107160, 0x80df43a532ce3782),
    ("cs", "! ! ! ! !", 112404, 0xa0eea4e039341448),
    ("cs", "! ! ! !", 112404, 0xa0eea4e039341448),
    ("hr", ": : : : :", 93108, 0xacade6b8227c0841),
    ("hr", ": : : :", 93108, 0xacade6b8227c0841),
    ("hr", "! ! ! ! !", 91238, 0x0f3527784c2a721c),
    ("hr", "! ! ! !", 91238, 0x0f3527784c2a721c),
    ("sr", ": : : : :", 93298, 0x2a619580bfda5aa9),
    ("sr", ": : : :", 93298, 0x2a619580bfda5aa9),
    ("sr", "! ! ! ! !", 90602, 0x256a018ca5d1dccb),
    ("sr", "! ! ! !", 90602, 0x256a018ca5d1dccb),
];

/// (voice, input, word and end events, then the sample total)
type EventCase = (&'static str, &'static str, &'static str);

const EVENT_CASES: &[EventCase] = &[
    ("fr", "¬ ¬ ¬ ¬ ¬", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:9:0:num=1 samples=71126"),
    ("fr", "¬ ¬ ¬ ¬", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:7:0:num=1 samples=71126"),
    ("uk", ", , , , ,", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:9:0:num=1 samples=35536"),
    ("uk", ", , , ,", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:7:0:num=1 samples=35536"),
    ("hi", "। । । । ।", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:9:0:num=1 samples=47322"),
    ("hi", "। । । ।", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:7:0:num=1 samples=47322"),
    ("hi", "॥ ॥ ॥ ॥ ॥", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:9:0:num=1 samples=53580"),
    ("hi", "॥ ॥ ॥ ॥", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:7:0:num=1 samples=53580"),
    ("cs", "! ! ! ! !", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:9:0:num=1 samples=56202"),
    ("cs", "! ! ! !", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:7:0:num=1 samples=56202"),
    ("hr", ": : : : :", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:9:0:num=1 samples=46554"),
    ("hr", ": : : :", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:7:0:num=1 samples=46554"),
    ("hr", "! ! ! ! !", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:9:0:num=1 samples=45619"),
    ("hr", "! ! ! !", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:7:0:num=1 samples=45619"),
    ("sr", ": : : : :", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:9:0:num=1 samples=46649"),
    ("sr", ": : : :", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:7:0:num=1 samples=46649"),
    ("sr", "! ! ! ! !", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:9:0:num=1 samples=45301"),
    ("sr", "! ! ! !", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 5:7:0:num=1 samples=45301"),
];

#[test]
fn phonemes_match_c() {
    for &(voice, input, x, ipa) in TEXT_CASES {
        assert_eq!(text(voice, "-x", input), x, "{voice} -x {input:?}");
        assert_eq!(text(voice, "--ipa", input), ipa, "{voice} --ipa {input:?}");
    }
}

#[test]
fn audio_matches_c() {
    for &(voice, input, len, hash) in WAV_CASES {
        let got = pcm(voice, input);
        assert_eq!((got.len(), fnv1a(&got)), (len, hash), "{voice} wav {input:?}");
    }
}

#[test]
fn events_match_c() {
    for &(voice, input, expected) in EVENT_CASES {
        let Some(got) = events(voice, input) else { return };
        assert_eq!(got, expected, "{voice} events {input:?}");
    }
}
