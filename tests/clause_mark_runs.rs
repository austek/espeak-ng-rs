// Runs of clause marks and of symbols read as words, against C f0fb11a8.  C keeps a mark that
// has no letter or digit before it in the clause text and reads it as a word, so `! ! !` is
// three one-character words whose events sit on the character before the next word.  A run of
// symbols with no blank between them (`¬¬¬¬`) is one word, and `_` is a blank to C whatever the
// language's list says about it.  Expected values come from `espeak-ng -v <voice> -q -x|--ipa
// <text>`, `-w out.wav` (PCM sample bytes and FNV-1a 64 hash) and the C library's word and end
// events (`probe.c evflat`, flags 0x1000).

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
    let dir = std::env::temp_dir().join(format!("clause_mark_runs_{}", std::process::id()));
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
    ("en", "! ! ! ! !", "_:'Ekskl@m,eIS@n_: _:'Ekskl@m,eIS@n_: _:'Ekskl@m,eIS@n_: _:'Ekskl@m,eIS@n_: _:'Ekskl@m,eIS@n_:", "ˈɛkskləmˌeɪʃən ˈɛkskləmˌeɪʃən ˈɛkskləmˌeɪʃən ˈɛkskləmˌeɪʃən ˈɛkskləmˌeɪʃən"),
    ("en", "! ! !", "_:'Ekskl@m,eIS@n_: _:'Ekskl@m,eIS@n_: _:'Ekskl@m,eIS@n_:", "ˈɛkskləmˌeɪʃən ˈɛkskləmˌeɪʃən ˈɛkskləmˌeɪʃən"),
    ("cs", "! !! ! ! !", "v'igR^itSn^i:k v'igR^itSn^i:k v'igR^itSn^i:k", "vˈiɡr̝itʃɲiːk vˈiɡr̝itʃɲiːk vˈiɡr̝itʃɲiːk"),
    ("uk", ",,,,, ,", "k'oma k'oma", "kˈoma kˈoma"),
    ("cs", "! ! !   ", "v'igR^itSn^i:k v'igR^itSn^i:k v'igR^itSn^i:k", "vˈiɡr̝itʃɲiːk vˈiɡr̝itʃɲiːk vˈiɡr̝itʃɲiːk"),
    ("hi", "। ।।। । ।", "p'u:rnwI*'a:m p'u:rnwI*'a:m p'u:rnwI*'a:m", "pˈuːrnwɪɾˈaːm pˈuːrnwɪɾˈaːm pˈuːrnwɪɾˈaːm"),
    ("fr", "¬¬¬¬", "tirE_fakyltatiftirE_fakyltatiftirE_fakyltat'if", "tiʁɛfakyltatiftiʁɛfakyltatiftiʁɛfakyltatˈif"),
    ("fr", "a ¬¬¬¬ b", "a tirE_fakyltatiftirE_fakyltatiftirE_fakyltat'if b'e", "a tiʁɛfakyltatiftiʁɛfakyltatiftiʁɛfakyltatˈif bˈe"),
    ("fr", "¬¬ ¬¬¬", "tirE_fakyltatiftirE_fakyltat'if tirE_fakyltatiftirE_fakyltatiftirE_fakyltat'if", "tiʁɛfakyltatiftiʁɛfakyltatˈif tiʁɛfakyltatiftiʁɛfakyltatiftiʁɛfakyltatˈif"),
    ("ru", "_ _ _ _", "", ""),
    ("ru", "я _ _ _ б", "'ja b'E", "ˈja bˈɛ"),
    ("ru", "_ a", "(en)'eI(ru)", "(en)ˈeɪ(ru)"),
    ("ru", "_", "", ""),
    ("mn", "_ _", "", ""),
    ("ky", "_ _ _ _ _", "", ""),
    ("ky", "я _ б", "j'a b", "jˈɑ b"),
    ("en", "_ _ _ _", "", ""),
    ("sk", "_ _", "", ""),
    ("it", "un:_.", "'Un", "ˈʊn"),
    ("it", "un:_…", "'Un", "ˈʊn"),
    ("fr", "un:_…", "'W~", "ˈœ̃"),
    ("fr", "un:_…his", "'W~\n(en)h'Iz(fr)", "ˈœ̃\n(en)hˈɪz(fr)"),
    ("it", "un:_ b", "Un b'i", "ʊn bˈi"),
    ("it", "un_…", "'un", "ˈun"),
    ("ky", "×÷", "kOb'Ojty:", "koebˈoejt[y:"),
];

/// (voice, input, PCM bytes, FNV-1a 64 of the PCM bytes); `mn` has a breath line, so its audio
/// differs on every C run and is left out.
type WavCase = (&'static str, &'static str, usize, u64);

const WAV_CASES: &[WavCase] = &[
    ("en", "! ! ! ! !", 206084, 0x922f5075af17ca1b),
    ("en", "! ! !", 129744, 0x1c11b152f4bcced4),
    ("cs", "! !! ! ! !", 112404, 0xa0eea4e039341448),
    ("uk", ",,,,, ,", 55278, 0x48d95f6263479801),
    ("cs", "! ! !   ", 112404, 0xa0eea4e039341448),
    ("hi", "। ।।। । ।", 94644, 0x13bbfd1c09b929f7),
    ("fr", "¬¬¬¬", 139368, 0x76a642730f2ba3bd),
    ("fr", "a ¬¬¬¬ b", 148056, 0xca3744683d26d99e),
    ("fr", "¬¬ ¬¬¬", 221182, 0xa2cadbf0bd57db8a),
    ("ru", "_ _ _ _", 308, 0xc10fa6af0a66b7b5),
    ("ru", "я _ _ _ б", 29480, 0x3308cf977a5778de),
    ("ru", "_ a", 24270, 0x1acf5dc8c0135aee),
    ("ru", "_", 308, 0xc10fa6af0a66b7b5),
    ("ky", "_ _ _ _ _", 308, 0xc10fa6af0a66b7b5),
    ("ky", "я _ б", 29044, 0x589c89fa78e15137),
    ("en", "_ _ _ _", 308, 0xc10fa6af0a66b7b5),
    ("sk", "_ _", 308, 0xc10fa6af0a66b7b5),
    ("it", "un:_.", 25560, 0x72c59c537b1cdc34),
    ("it", "un:_…", 22252, 0xf2406d0f21209674),
    ("fr", "un:_…", 17248, 0xaa53c88eeea40720),
    ("fr", "un:_…his", 44458, 0xe0b179cff4054742),
    ("it", "un:_ b", 30764, 0x89d3d7e9e27dc43a),
    ("it", "un_…", 23250, 0x421cfdea40c8595f),
    ("ky", "×÷", 43066, 0x81ccfc29f47c9b0e),
];

/// (voice, input, word and end events, then the sample total)
type EventCase = (&'static str, &'static str, &'static str);

const EVENT_CASES: &[EventCase] = &[
    ("en", "! ! ! ! !", "1:2:1:num=1 1:4:1:num=2 1:6:1:num=3 1:8:1:num=4 1:9:1:num=5 5:9:0:num=1 samples=103042"),
    ("en", "! ! !", "1:2:1:num=1 1:4:1:num=2 1:5:1:num=3 5:5:0:num=1 samples=64872"),
    ("cs", "! !! ! ! !", "1:2:1:num=1 1:5:1:num=2 1:7:1:num=3 5:10:0:num=1 samples=56202"),
    ("uk", ",,,,, ,", "1:6:1:num=1 1:7:1:num=2 5:7:0:num=1 samples=27639"),
    ("cs", "! ! !   ", "1:2:1:num=1 1:4:1:num=2 1:8:1:num=3 5:8:0:num=1 samples=56202"),
    ("hi", "। ।।। । ।", "1:2:1:num=1 1:6:1:num=2 1:8:1:num=3 5:9:0:num=1 samples=47322"),
    ("fr", "¬¬¬¬", "1:1:4:num=1 5:4:0:num=1 samples=69684"),
    ("fr", "a ¬¬¬¬ b", "1:1:1:num=1 1:3:4:num=2 1:8:1:num=3 5:8:0:num=1 samples=74028"),
    ("fr", "¬¬ ¬¬¬", "1:1:2:num=1 1:4:3:num=2 5:6:0:num=1 samples=110591"),
    ("ru", "_ _ _ _", "5:7:0:num=1 samples=154"),
    ("ru", "я _ _ _ б", "1:1:1:num=1 1:3:1:num=2 5:9:0:num=1 samples=14740"),
    ("ru", "_ a", "1:1:1:num=1 5:3:0:num=1 samples=12135"),
    ("ru", "_", "5:1:0:num=1 samples=154"),
    ("mn", "_ _", "5:3:0:num=1 samples=198"),
    ("ky", "_ _ _ _ _", "5:9:0:num=1 samples=154"),
    ("ky", "я _ б", "1:1:1:num=1 1:3:1:num=2 5:5:0:num=1 samples=14522"),
    ("en", "_ _ _ _", "5:7:0:num=1 samples=154"),
    ("sk", "_ _", "5:3:0:num=1 samples=154"),
    ("it", "un:_.", "1:1:2:num=1 5:5:0:num=1 samples=12780"),
    ("it", "un:_…", "1:1:2:num=1 5:5:0:num=1 samples=11126"),
    ("fr", "un:_…", "1:1:2:num=1 5:5:0:num=1 samples=8624"),
    ("fr", "un:_…his", "1:1:2:num=1 5:5:0:num=1 1:6:3:num=3 5:8:0:num=1 samples=22229"),
    ("it", "un:_ b", "1:1:2:num=1 1:4:1:num=2 5:6:0:num=1 samples=15382"),
    ("it", "un_…", "1:1:2:num=1 5:4:0:num=1 samples=11625"),
    ("ky", "×÷", "1:1:2:num=1 5:2:0:num=1 samples=21533"),
];

/// C announces `_` through `--punct` before `TranslateClause` turns it into a blank.
#[test]
fn announced_underscore_matches_c() {
    let run = |voice: &str, punct: &str, input: &str| {
        let out = cli(voice, &["-q", "-x", punct]).args(["--", input]).output().expect("run");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    assert_eq!(run("en", "--punct=_", "a _ b"), "a# 'Vnd3sk,o@ b'i:");
    assert_eq!(run("ru", "--punct", "я _ _ б"), "'ja pV#ttS;'8rk;ivV#n;ijI pV#ttS;'8rk;ivV#n;ijI b'E");
}

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
