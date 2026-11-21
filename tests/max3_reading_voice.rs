// C marks every dictionary hit on a one-character word that is not a letter with `FLAG_MAX3`
// (`LookupDict2`), `$max3` in the list or not, and counts the repeats in the translator that
// reads the symbol: `~ ~ ~ ~` is three tildes in de, fr, ca, ru, uk, pl and it, whose lists do
// not carry `$max3` on `~`.  A replacement text (`¢ cent` in the emoji list) is translated again,
// which ends the run, and a symbol read through the English translator is never counted.
// Expected values come from `espeak-ng -v <voice> -q -x|--ipa <text>`, `-w out.wav` (sample count
// and FNV-1a 64 hash of the PCM bytes) and the C library's word and end events, C f0fb11a8.

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
    let dir = std::env::temp_dir().join(format!("max3_reading_voice_{}", std::process::id()));
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
    ("ca", "& & & & &", "'an 'an 'an", "ˈan ˈan ˈan"),
    ("ca", "~ a ~ ~ ~ ~ ~", "t'idl^@ @ t'idl^@ t'idl^@ t'idl^@", "tˈidʎə ə tˈidʎə tˈidʎə tˈidʎə"),
    ("ca", "~ ~ ~ ~", "t'idl^@ t'idl^@ t'idl^@", "tˈidʎə tˈidʎə tˈidʎə"),
    ("ca", "~ ~ ~. ~ ~ ~", "t'idl^@ t'idl^@ t'idl^@", "tˈidʎə tˈidʎə tˈidʎə"),
    (
        "cmn",
        "& & & & &",
        "(en)'a55mp@55s,a55nd_| 'a55mp@55s,a55nd_| 'a55mp@55s,a55nd_| 'a55mp@55s,a55nd_| 'a55mp@55s,a55nd(cmn)_|",
        "(en)ˈa5mpə5sˌa5nd ˈa5mpə5sˌa5nd ˈa5mpə5sˌa5nd ˈa5mpə5sˌa5nd ˈa5mpə5sˌa5nd(cmn)",
    ),
    (
        "cmn",
        "~ ~ ~ ~",
        "(en)t'I55ld@55_| t'I55ld@55_| t'I55ld@55_| t'I55ld@55(cmn)_|",
        "(en)tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5(cmn)",
    ),
    (
        "cmn",
        "~ ~ ~. ~ ~ ~",
        "(en)t'I55ld@55_| t'I55ld@55_| t'I55ld@55_| t'I55ld@55_| t'I55ld@55_| t'I55ld@55(cmn)_|",
        "(en)tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5(cmn)",
    ),
    ("de", "& & & & &", "_!'Unt _!'Unt _!'Unt", "ˈʊnt ˈʊnt ˈʊnt"),
    ("de", "~ a ~ ~ ~ ~ ~", "t'Ild@ _!'A: t'Ild@ t'Ild@ t'Ild@", "tˈɪldə ˈɑː tˈɪldə tˈɪldə tˈɪldə"),
    ("de", "~ ~ ~ ~", "t'Ild@ t'Ild@ t'Ild@", "tˈɪldə tˈɪldə tˈɪldə"),
    ("de", "~ ~ ~. ~ ~ ~", "t'Ild@ t'Ild@ t'Ild@", "tˈɪldə tˈɪldə tˈɪldə"),
    ("de", "¢ ¢ ¢ ¢ ¢", "s'Ent s'Ent s'Ent s'Ent s'Ent", "sˈɛnt sˈɛnt sˈɛnt sˈɛnt sˈɛnt"),
    ("es", "& & & & &", ",ampErs'ant ,ampErs'ant ,ampErs'ant", "ˌampɛɾsˈant ˌampɛɾsˈant ˌampɛɾsˈant"),
    ("es", "~ ~ ~ ~", "(en)t'Ild@ t'Ild@ t'Ild@ t'Ild@(es)", "(en)tˈɪldə tˈɪldə tˈɪldə tˈɪldə(es)"),
    (
        "es",
        "~ ~ ~. ~ ~ ~",
        "(en)t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@(es)",
        "(en)tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə(es)",
    ),
    ("fr", "~ a ~ ~ ~ ~ ~", "tild'e a tild'e tild'e tild'e", "tildˈe a tildˈe tildˈe tildˈe"),
    ("fr", "~ ~ ~ ~", "tild'e tild'e tild'e", "tildˈe tildˈe tildˈe"),
    ("fr", "~ ~ ~. ~ ~ ~", "tild'e tild'e tild'e", "tildˈe tildˈe tildˈe"),
    ("fr", "¢ ¢ ¢ ¢ ¢", "s'Ent s'Ent s'Ent", "sˈɛnt sˈɛnt sˈɛnt"),
    ("it", "& & & & &", "'and 'and 'and", "ˈand ˈand ˈand"),
    ("it", "~ a ~ ~ ~ ~ ~", "t'ilde_| a t'ilde t'ilde t'ilde", "tˈilde a tˈilde tˈilde tˈilde"),
    ("it", "~ ~ ~ ~", "t'ilde t'ilde t'ilde", "tˈilde tˈilde tˈilde"),
    ("it", "~ ~ ~. ~ ~ ~", "t'ilde t'ilde t'ilde", "tˈilde tˈilde tˈilde"),
    (
        "ja",
        "& & & & &",
        "(en)'amp@s,and 'amp@s,and 'amp@s,and 'amp@s,and 'amp@s,and(ja)",
        "(en)ˈampəsˌand ˈampəsˌand ˈampəsˌand ˈampəsˌand ˈampəsˌand(ja)",
    ),
    ("ja", "~ ~ ~ ~", "(en)t'Ild@ t'Ild@ t'Ild@ t'Ild@(ja)", "(en)tˈɪldə tˈɪldə tˈɪldə tˈɪldə(ja)"),
    (
        "ja",
        "~ ~ ~. ~ ~ ~",
        "(en)t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@(ja)",
        "(en)tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə(ja)",
    ),
    (
        "ko",
        "& & & & &",
        "(en)'amp@s,and 'amp@s,and 'amp@s,and 'amp@s,and 'amp@s,and(ko)",
        "(en)ˈampəsˌand ˈampəsˌand ˈampəsˌand ˈampəsˌand ˈampəsˌand(ko)",
    ),
    ("ko", "~ ~ ~ ~", "(en)t'Ild@ t'Ild@ t'Ild@ t'Ild@(ko)", "(en)tˈɪldə tˈɪldə tˈɪldə tˈɪldə(ko)"),
    (
        "ko",
        "~ ~ ~. ~ ~ ~",
        "(en)t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@(ko)",
        "(en)tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə(ko)",
    ),
    ("nl", "& & & & &", "_|'En _|'En _|'En", "ˈɛn ˈɛn ˈɛn"),
    ("nl", "~ a ~ ~ ~ ~ ~", "_!'a:", "ˈaː"),
    ("nl", "¢ ¢ ¢ ¢ ¢", "s'Ent s'Ent s'Ent s'Ent s'Ent", "sˈɛnt sˈɛnt sˈɛnt sˈɛnt sˈɛnt"),
    ("pl", "& & & & &", "'and 'and 'and", "ˈand ˈand ˈand"),
    ("pl", "~ ~ ~ ~", "t'ylda t'ylda t'ylda", "tˈɨlda tˈɨlda tˈɨlda"),
    ("pl", "~ ~ ~. ~ ~ ~", "t'ylda t'ylda t'ylda", "tˈɨlda tˈɨlda tˈɨlda"),
    ("pl", "¢ ¢ ¢ ¢ ¢", "ts'Ent ts'Ent ts'Ent", "tsˈɛnt tsˈɛnt tsˈɛnt"),
    ("pt", "& & & & &", "_'e_ _'e_ _'e_", "ˈe ˈe ˈe"),
    ("pt", "~ a ~ ~ ~ ~ ~", "'&", "ˈɐ"),
    ("pt", "¢ ¢ ¢ ¢ ¢", "s,eNt'avUs# s,eNt'avUs# s,eNt'avUs#", "sˌeŋtˈavʊʃ sˌeŋtˈavʊʃ sˌeŋtˈavʊʃ"),
    ("ru", "& & & & &", "amp;irs'And amp;irs'And amp;irs'And", "ampʲirsˈɑnd ampʲirsˈɑnd ampʲirsˈɑnd"),
    ("ru", "~ ~ ~ ~", "t;'ilda# t;'ilda# t;'ilda#", "tʲˈiɭda tʲˈiɭda tʲˈiɭda"),
    ("ru", "~ ~ ~. ~ ~ ~", "t;'ilda# t;'ilda# t;'ilda#", "tʲˈiɭda tʲˈiɭda tʲˈiɭda"),
    ("uk", "& & & & &", "ampeRs'and ampeRs'and ampeRs'and", "ampersˈand ampersˈand ampersˈand"),
    ("uk", "~ ~ ~ ~", "t'Il^da t'Il^da t'Il^da", "tˈɪʎda tˈɪʎda tˈɪʎda"),
    ("uk", "~ ~ ~. ~ ~ ~", "t'Il^da t'Il^da t'Il^da", "tˈɪʎda tˈɪʎda tˈɪʎda"),
];

/// (voice, input, PCM bytes, PCM hash)
type WavCase = (&'static str, &'static str, usize, u64);

const WAV_CASES: &[WavCase] = &[
    ("ca", "& & & & &", 36516, 0xfd3f8a967ec215b6),
    ("ca", "~ a ~ ~ ~ ~ ~", 77668, 0x37b35da2f9970ae0),
    ("ca", "~ ~ ~ ~", 59906, 0xb35679db2d40ee86),
    ("ca", "~ ~ ~. ~ ~ ~", 59906, 0xb35679db2d40ee86),
    ("de", "& & & & &", 49882, 0x525d4f72ec400563),
    ("de", "~ a ~ ~ ~ ~ ~", 85550, 0x35d3e5f7301c53aa),
    ("de", "~ ~ ~ ~", 61638, 0x89cf59c9849de1a0),
    ("de", "~ ~ ~. ~ ~ ~", 61638, 0x89cf59c9849de1a0),
    ("de", "¢ ¢ ¢ ¢ ¢", 86230, 0x7d1322e8c3adae22),
    ("es", "& & & & &", 98162, 0x7ff38d937045c038),
    ("es", "~ a ~ ~ ~ ~ ~", 103752, 0x88dd1c4731f3d5be),
    ("es", "~ ~ ~ ~", 72052, 0x32bdb927c4527db0),
    ("es", "~ ~ ~. ~ ~ ~", 100138, 0x4cd5a0604e47c8a),
    ("fr", "~ a ~ ~ ~ ~ ~", 78424, 0x450992bf4f9cb10f),
    ("fr", "~ ~ ~ ~", 60074, 0xd328f158ca818ab2),
    ("fr", "~ ~ ~. ~ ~ ~", 60074, 0xd328f158ca818ab2),
    ("fr", "¢ ¢ ¢ ¢ ¢", 58724, 0xc5201cba6de8a1f7),
    ("it", "& & & & &", 43314, 0x53cbe7ada5fde8ae),
    ("it", "~ a ~ ~ ~ ~ ~", 77940, 0xbb7ccd4b06c73c08),
    ("it", "~ ~ ~ ~", 60614, 0x490c280994168628),
    ("it", "~ ~ ~. ~ ~ ~", 60614, 0x490c280994168628),
    ("ja", "& & & & &", 138014, 0x27e1bb7f45a20636),
    ("ja", "~ ~ ~ ~", 78122, 0xddb9d8a308008964),
    ("ja", "~ ~ ~. ~ ~ ~", 108908, 0x99ea1bbfc2247c46),
    ("ko", "& & & & &", 138270, 0xf161a56bf2d710ba),
    ("ko", "~ ~ ~ ~", 78252, 0x81da0bdc5caa277a),
    ("ko", "~ ~ ~. ~ ~ ~", 108860, 0xb1af6d7c566a6356),
    ("nl", "& & & & &", 39888, 0x45f0cb05f227b088),
    ("nl", "~ a ~ ~ ~ ~ ~", 25304, 0x98fb9d8e08166dad),
    ("nl", "¢ ¢ ¢ ¢ ¢", 89206, 0x2ef3feffe523444d),
    ("pl", "& & & & &", 43362, 0x6c967f8c36a64620),
    ("pl", "~ ~ ~ ~", 66084, 0xa6f8b1a3f300eae1),
    ("pl", "~ ~ ~. ~ ~ ~", 66084, 0xa6f8b1a3f300eae1),
    ("pl", "¢ ¢ ¢ ¢ ¢", 65198, 0xe5108c29a6d3244c),
    ("pt", "& & & & &", 37448, 0x3040b3bc2dba22e8),
    ("pt", "¢ ¢ ¢ ¢ ¢", 95234, 0x9d5364432da5e0a5),
    ("ru", "& & & & &", 85556, 0x348a9cd3dd3598a6),
    ("ru", "~ ~ ~ ~", 43486, 0x51c711aa402263ba),
    ("ru", "~ ~ ~. ~ ~ ~", 43486, 0x51c711aa402263ba),
    ("uk", "& & & & &", 123714, 0x7944b536e4e6188d),
    ("uk", "~ ~ ~ ~", 87536, 0x5ddc0cfb98d6d521),
    ("uk", "~ ~ ~. ~ ~ ~", 87536, 0x5ddc0cfb98d6d521),
    ("uk", "¢ ¢ ¢ ¢ ¢", 19006, 0xbfc2318761c0809d),
];

/// (voice, input, word and end events, then the sample total)
type EventCase = (&'static str, &'static str, &'static str);

const EVENT_CASES: &[EventCase] = &[
    ("ca", "~ ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:7:0:num=1 samples=29953"),
    ("ca", "~ ~ ~. ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:12:0:num=1 samples=29953"),
    ("de", "~ ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:7:0:num=1 samples=30819"),
    ("de", "~ ~ ~. ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:12:0:num=1 samples=30819"),
    ("es", "~ ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 1:7:1:num=4 5:7:0:num=1 samples=36026"),
    (
        "es",
        "~ ~ ~. ~ ~ ~",
        "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 1:8:1:num=4 1:10:1:num=5 1:12:1:num=6 5:12:0:num=1 samples=50069",
    ),
    ("fr", "~ ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:7:0:num=1 samples=30037"),
    ("fr", "~ ~ ~. ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:12:0:num=1 samples=30037"),
    ("it", "~ ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:7:0:num=1 samples=30307"),
    ("it", "~ ~ ~. ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:12:0:num=1 samples=30307"),
    ("ja", "~ ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 1:7:1:num=4 5:7:0:num=1 samples=39061"),
    (
        "ja",
        "~ ~ ~. ~ ~ ~",
        "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 1:8:1:num=4 1:10:1:num=5 1:12:1:num=6 5:12:0:num=1 samples=54454",
    ),
    ("ko", "~ ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 1:7:1:num=4 5:7:0:num=1 samples=39126"),
    (
        "ko",
        "~ ~ ~. ~ ~ ~",
        "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 1:8:1:num=4 1:10:1:num=5 1:12:1:num=6 5:12:0:num=1 samples=54430",
    ),
    ("pl", "~ ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:7:0:num=1 samples=33042"),
    ("pl", "~ ~ ~. ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:12:0:num=1 samples=33042"),
    ("ru", "~ ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:7:0:num=1 samples=21743"),
    ("ru", "~ ~ ~. ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:12:0:num=1 samples=21743"),
    ("uk", "~ ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:7:0:num=1 samples=43768"),
    ("uk", "~ ~ ~. ~ ~ ~", "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 5:12:0:num=1 samples=43768"),
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
