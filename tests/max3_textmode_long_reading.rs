// Two limits on C's repeat count for one-character words (`LookupDict2`, `LookupDictList`):
// cmn and yue store `FLAG_TEXTMODE` inverted (`langopts.textmode`), so their `*_emoji` entries
// are replacement texts, which are translated again and never limited; and `phonemes_repeat` is
// a `char[20]`, so a reading of 20 bytes or more never equals the stored copy and is never
// limited (uk `\` is `ob'ERnEna||kos'a||R'Iska`).
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
    let dir = std::env::temp_dir().join(format!("max3_textmode_long_{}", std::process::id()));
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
    (
        "uk",
        "\\ \\ \\ \\ \\",
        "ob'ERnEna kos'a R'Iska ob'ERnEna kos'a R'Iska ob'ERnEna kos'a R'Iska ob'ERnEna kos'a R'Iska ob'ERnEna kos'a R'Iska",
        "obˈɛrnɛna kosˈa rˈɪska obˈɛrnɛna kosˈa rˈɪska obˈɛrnɛna kosˈa rˈɪska obˈɛrnɛna kosˈa rˈɪska obˈɛrnɛna kosˈa rˈɪska",
    ),
    ("uk", "€ € € € €", "j'EBRo j'EBRo j'EBRo j'EBRo j'EBRo", "jˈɛβro jˈɛβro jˈɛβro jˈɛβro jˈɛβro"),
    (
        "uk",
        "™ ™ ™ ™ ™",
        "t[oRh'oBEl^na m'aRka t[oRh'oBEl^na m'aRka t[oRh'oBEl^na m'aRka t[oRh'oBEl^na m'aRka t[oRh'oBEl^na m'aRka",
        "t̪orhˈoβɛʎna mˈarka t̪orhˈoβɛʎna mˈarka t̪orhˈoβɛʎna mˈarka t̪orhˈoβɛʎna mˈarka t̪orhˈoβɛʎna mˈarka",
    ),
    (
        "uk",
        "≠ ≠ ≠ ≠ ≠",
        "nER'i:Bn^i:st nER'i:Bn^i:st nER'i:Bn^i:st nER'i:Bn^i:st nER'i:Bn^i:st",
        "nɛrˈiːβɲiːst nɛrˈiːβɲiːst nɛrˈiːβɲiːst nɛrˈiːβɲiːst nɛrˈiːβɲiːst",
    ),
    (
        "cmn",
        "~ ~ ~ ~ ~",
        "(en)t'I55ld@55_| t'I55ld@55_| t'I55ld@55_| t'I55ld@55_| t'I55ld@55(cmn)_|",
        "(en)tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5(cmn)",
    ),
    (
        "yue",
        "\\ \\ \\ \\ \\",
        "(en)b'a1ksla1S_| b'a1ksla1S_| b'a1ksla1S_| b'a1ksla1S_| b'a1ksla1S(yue)_|",
        "(en)bˈa1ksla1ʃ bˈa1ksla1ʃ bˈa1ksla1ʃ bˈa1ksla1ʃ bˈa1ksla1ʃ(yue)",
    ),
    (
        "yue",
        "~ ~ ~ ~ ~",
        "(en)t'I1ld@1_| t'I1ld@1_| t'I1ld@1_| t'I1ld@1_| t'I1ld@1(yue)_|",
        "(en)tˈɪ1ldə1 tˈɪ1ldə1 tˈɪ1ldə1 tˈɪ1ldə1 tˈɪ1ldə1(yue)",
    ),
    (
        "yue",
        "© © © © © ©",
        "b'aa2n_| k'yu4n_| b'aa2n_| k'yu4n_| b'aa2n_| k'yu4n_| b'aa2n_| k'yu4n_| b'aa2n_| k'yu4n_| b'aa2n_| k'yu4n_|",
        "bˈaa2n kˈyu4n bˈaa2n kˈyu4n bˈaa2n kˈyu4n bˈaa2n kˈyu4n bˈaa2n kˈyu4n bˈaa2n kˈyu4n",
    ),
    (
        "yue",
        "€ € € € €",
        "N'au1_| l'o4_| N'au1_| l'o4_| N'au1_| l'o4_| N'au1_| l'o4_| N'au1_| l'o4_|",
        "ŋˈau1 lˈo4 ŋˈau1 lˈo4 ŋˈau1 lˈo4 ŋˈau1 lˈo4 ŋˈau1 lˈo4",
    ),
    (
        "yue",
        "™ ™ ™ ™ ™",
        "s'oeng1_| b'iu1_| s'oeng1_| b'iu1_| s'oeng1_| b'iu1_| s'oeng1_| b'iu1_| s'oeng1_| b'iu1_|",
        "sˈoenɡ1 bˈiu1 sˈoenɡ1 bˈiu1 sˈoenɡ1 bˈiu1 sˈoenɡ1 bˈiu1 sˈoenɡ1 bˈiu1",
    ),
];

/// (voice, input, PCM bytes, PCM hash)
type WavCase = (&'static str, &'static str, usize, u64);

const WAV_CASES: &[WavCase] = &[
    ("uk", "\\ \\ \\ \\ \\", 341338, 0x08e4cf0f05dff592),
    ("uk", "€ € € € €", 118148, 0x80fb9a2738c703e9),
    ("uk", "™ ™ ™ ™ ™", 308580, 0xac3193b81eddef9d),
    ("uk", "≠ ≠ ≠ ≠ ≠", 206818, 0x6f9f8420c656dd9b),
    ("yue", "\\ \\ \\ \\ \\", 131344, 0xcbcae4f4d091e3a7),
    ("yue", "~ ~ ~ ~ ~", 90820, 0x0ad86a4ecc8f6711),
    ("yue", "€ € € € €", 95666, 0x61a3f2aaa0fb9a85),
    ("yue", "™ ™ ™ ™ ™", 109110, 0xa69cbe3153229b83),
    ("yue", "≠ ≠ ≠ ≠ ≠", 238744, 0xd37bcda32b13700a),
];

/// (voice, input, word and end events, then the sample total)
type EventCase = (&'static str, &'static str, &'static str);

const EVENT_CASES: &[EventCase] = &[
    (
        "uk",
        "€ € € € €",
        "1:1:1:num=1 1:3:1:num=2 1:5:1:num=3 1:7:1:num=4 1:9:1:num=5 5:9:0:num=1 samples=59074",
    ),
    (
        "yue",
        "€ € € € €",
        "1:1:1:num=1 1:2:1:num=2 1:3:1:num=3 1:4:1:num=4 1:5:1:num=5 1:6:1:num=6 1:7:1:num=7 1:8:1:num=8 1:9:1:num=9 1:10:1:num=10 5:9:0:num=1 samples=47833",
    ),
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
