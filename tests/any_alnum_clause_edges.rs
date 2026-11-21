// tests/any_alnum_clause_edges.rs
//
// Edges of C's `ReadClause` `any_alnum` rule.  Expected values are what C espeak-ng master
// (f0fb11a8) produces: `-q -x`, `-q --ipa`, the PCM of `-w` (byte count and FNV-1a 64 hash) and
// the word/sentence/end events (FNV-1a 64 hash of the transcript lines
// `ev <pos> <len> <ms> <sample> <type> num=<n>` and `samples=<n>`).
//
// - `\x01V` returns `CLAUSE_VOICE`: the clause ends there, the voice name is not text and the
//   flag starts clear.
// - A blanked `.` followed by words ends its clause with `CLAUSE_EOF`, which is no full stop.
// - A `.` that no blank follows (`.-`) is not a clause end, so it is not blanked.
// - A language switch closes before the pause of a spaced dash.
//
// Each row's last field says which of the four this port reproduces; the others differ from C
// for reasons outside the rule (the `CLAUSE_VOICE` pause, Mandarin clause pauses, the word
// event of an English letter inside Japanese, word breaks after a switched word).

use std::path::Path;
use std::process::Command;

use espeak_ng::{EspeakNg, EventKind};

const X: u8 = 1;
const IPA: u8 = 2;
const WAV: u8 = 4;
const EVENTS: u8 = 8;
const ALL: u8 = X | IPA | WAV | EVENTS;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn cli(voice: &str) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.args(["-v", voice]);
    cmd
}

fn text(voice: &str, flag: &str, input: &str) -> String {
    let out = cli(voice).args(["-q", flag, "--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(voice: &str, input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("any_alnum_edges_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(format!("{voice}{input}").as_bytes())));
    let status = cli(voice).arg("-w").arg(&path).args(["--", input]).status().expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

fn transcript_hash(voice: &str, input: &str) -> u64 {
    let engine = EspeakNg::builder().voice(voice).data_dir(Path::new("espeak-ng-data")).build().expect("engine");
    let (samples, _rate, events) = engine.synth_with_events(input).expect("synth");
    let mut out = String::new();
    for ev in &events {
        let kind = match &ev.kind {
            EventKind::Word(n) => format!("1 num={n}"),
            EventKind::Sentence(n) => format!("2 num={n}"),
            EventKind::End(n) => format!("5 num={n}"),
            _ => continue,
        };
        out.push_str(&format!("ev {} {} {} {} {kind}\n", ev.text_position, ev.length, ev.audio_position_ms, ev.sample));
    }
    out.push_str(&format!("samples={}\n", samples.len()));
    fnv1a(out.as_bytes())
}

/// (voice, input, `-x`, `--ipa`, PCM bytes, PCM hash, transcript hash, fields to check)
type Row = (&'static str, &'static str, &'static str, &'static str, usize, u64, u64, u8);

const ROWS: &[Row] = &[
    ("en", "a \u{1}V Ok", "'eI\n,oUk'eI", "ˈeɪ\nˌəʊkˈeɪ", 40376, 0xc3082326fc1a0654, 0x95511cc6001f4fb4, X | IPA),
    ("en", "a \u{1}V . Ok", "'eI\n,oUk'eI", "ˈeɪ\nˌəʊkˈeɪ", 40376, 0xc3082326fc1a0654, 0xf9601a335da32380, X | IPA),
    ("en", "~ . ok", "t'Ild@r- ,oUk'eI", "tˈɪldəɹ ˌəʊkˈeɪ", 47448, 0xe1de75e47c3f542f, 0xff299e8e7fdde394, ALL),
    ("en", "% . ok", "p3s'Ent ,oUk'eI", "pəsˈɛnt ˌəʊkˈeɪ", 51398, 0x27f73b37fb8c651d, 0x3c0cedafd0da1620, ALL),
    ("en", "... . ok", ",oUk'eI", "ˌəʊkˈeɪ", 31942, 0xb0b20fb136fdcae1, 0x715588fb50ab6d79, ALL),
    ("en", "~ .) Ok", "t'Ild@_:_: ,oUk'eI", "tˈɪldə ˌəʊkˈeɪ", 53148, 0x7a4ba101adf2d493, 0x9a6b69b2d345dca8, ALL),
    ("en", "-. Ok", ",oUk'eI", "ˌəʊkˈeɪ", 31942, 0xb0b20fb136fdcae1, 0x01e6eab21e354889, ALL),
    ("en", "~\u{ad}. Ok", "t'Ild@r- ,oUk'eI", "tˈɪldəɹ ˌəʊkˈeɪ", 47448, 0xe1de75e47c3f542f, 0xff299e8e7fdde394, X | IPA | WAV),
    ("en", "~\u{200d}. Ok", "t'Ild@r- ,oUk'eI", "tˈɪldəɹ ˌəʊkˈeɪ", 47448, 0xe1de75e47c3f542f, 0xa1a5a9f49e3f0eb3, ALL),
    ("en", "~\u{f0b}. Ok", "t'Ild@r- ,oUk'eI", "tˈɪldəɹ ˌəʊkˈeɪ", 47448, 0xe1de75e47c3f542f, 0xff299e8e7fdde394, ALL),
    ("en", "+ . - . =", "pl'Vs_:_: _:'i:kw@Lz_:", "plˈʌs ˈiːkwəlz", 58892, 0xb2c17028327c98cf, 0x60aa71c3046d10b4, X | IPA | WAV),
    ("en", "a - b", "a#_:_: b'i:", "ɐ bˈiː", 33824, 0xc95c195a3258690a, 0x8b03e99449d21b90, ALL),
    ("en", "~ . - . ~", "t'Ild@_:_: t'Ild@", "tˈɪldə tˈɪldə", 53196, 0x3e4f6bdc0e684880, 0xcd918ef8132219b9, ALL),
    ("de", "a \u{1}V Ok", "_!'A:\n_|o:k'e:", "ˈɑː\noːkˈeː", 40676, 0xab347ce91780caf2, 0xabb51115f6d3b496, X | IPA),
    ("de", "a \u{1}V . Ok", "_!'A:\n_|o:k'e:", "ˈɑː\noːkˈeː", 40676, 0xab347ce91780caf2, 0xc7a2a93108555e46, X | IPA),
    ("de", "~ . ok", "t'Ild@ _|o:k'e:", "tˈɪldə oːkˈeː", 45112, 0x3905f1f9c1150057, 0x228aef2d549528c2, ALL),
    ("de", "% . ok", "pro:ts'Ent _|o:k'e:", "pɾoːtsˈɛnt oːkˈeː", 53852, 0x183ba539cc9b1ddf, 0x4c7911aeea69a32e, ALL),
    ("de", "... . ok", "_|o:k'e:", "oːkˈeː", 30062, 0x436bb074a868862b, 0x4a933bd609c49e73, ALL),
    ("de", "~ .) Ok", "t'Ild@_:_: _|o:k'e:", "tˈɪldə oːkˈeː", 50356, 0x60c1a9a073148b58, 0x3fa80e582e1ea2cf, ALL),
    ("de", "-. Ok", "_|o:k'e:", "oːkˈeː", 30062, 0x436bb074a868862b, 0x3ae1db56cdb22fc3, ALL),
    ("de", "~\u{ad}. Ok", "t'Ild@ _|o:k'e:", "tˈɪldə oːkˈeː", 45112, 0x3905f1f9c1150057, 0x228aef2d549528c2, X | IPA | WAV),
    ("de", "~\u{200d}. Ok", "t'Ild@ _|o:k'e:", "tˈɪldə oːkˈeː", 45112, 0x3905f1f9c1150057, 0xaf229a4d2901ef6d, ALL),
    ("de", "~\u{f0b}. Ok", "t'Ild@ _|o:k'e:", "tˈɪldə oːkˈeː", 45112, 0x3905f1f9c1150057, 0x228aef2d549528c2, ALL),
    ("de", "~ .- Ok", "t'Ild@ _|o:k'e:", "tˈɪldə oːkˈeː", 45112, 0x3905f1f9c1150057, 0x4f7915e512c7ffe0, ALL),
    ("de", "a .- Ok", "_!'A: _|o:k'e:", "ˈɑː oːkˈeː", 38416, 0x03f2da62df1d2f93, 0xd772798d30013bc4, ALL),
    ("de", "+ . - . =", "pl'Us_:_: gl'aIC", "plˈʊs ɡlˈaɪç", 46658, 0xcf8019f1beafb418, 0x0eeaddd1b6550feb, ALL),
    ("de", "a - b", "_!'A:_:_: b'e:", "ˈɑː bˈeː", 40560, 0x186e1c57d0a6fbc5, 0xd637ae94084608bb, ALL),
    ("de", "~ . - . ~", "t'Ild@_:_: t'Ild@", "tˈɪldə tˈɪldə", 52230, 0x114feddf9e3b06f5, 0x151371518675d298, ALL),
    ("fr", "a \u{1}V Ok", "'a\nok'e", "ˈa\nokˈe", 31068, 0x0cf682cc9b1f3d61, 0x1f73b6f735e9417c, X | IPA),
    ("fr", "a \u{1}V . Ok", "'a\nok'e", "ˈa\nokˈe", 31068, 0x0cf682cc9b1f3d61, 0xa6519ae35d76c6f4, X | IPA),
    ("fr", "~ . ok", "tild'e ok'e", "tildˈe okˈe", 41310, 0x86e36fd2ee759cf5, 0x12f0dbe8e0ef5c9e, ALL),
    ("fr", "% . ok", "purs'A~ ok'e", "puʁsˈɑ̃ okˈe", 42066, 0x71f5178c7db9b5bd, 0xd7e0fbc933ace335, ALL),
    ("fr", "... . ok", "ok'e", "okˈe", 26898, 0x5c377c3e47179e0f, 0x901084e148e60a9f, ALL),
    ("fr", "~ .) Ok", "tild'e_:_: ok'e", "tildˈe okˈe", 46550, 0xd9152d29c1326f1c, 0x401be05938893fbd, ALL),
    ("fr", "-. Ok", "ok'e", "okˈe", 26898, 0x5c377c3e47179e0f, 0x4d99a1ee06a3582f, ALL),
    ("fr", "~\u{ad}. Ok", "tild'e ok'e", "tildˈe okˈe", 41310, 0x86e36fd2ee759cf5, 0x12f0dbe8e0ef5c9e, X | IPA | WAV),
    ("fr", "~\u{200d}. Ok", "tild'e ok'e", "tildˈe okˈe", 41310, 0x86e36fd2ee759cf5, 0xd18cae44b3492bcb, ALL),
    ("fr", "~\u{f0b}. Ok", "tild'e ok'e", "tildˈe okˈe", 41310, 0x86e36fd2ee759cf5, 0x12f0dbe8e0ef5c9e, ALL),
    ("fr", "~ .- Ok", "tild'e ok'e", "tildˈe okˈe", 41310, 0x86e36fd2ee759cf5, 0x0e04766bc874c754, ALL),
    ("fr", "+ . - . =", "pl'ys_:_:", "plˈys", 34126, 0x174ba45d22a51ea7, 0xc10f092ef6b5f1dd, IPA),
    ("fr", "a - b", "a_:_: b'e", "a bˈe", 30026, 0x1d3bf2738525a6af, 0x6cf6ec3f8c5148bb, ALL),
    ("fr", "~ . - . ~", "tild'e_:_: tild'e", "tildˈe tildˈe", 51434, 0xca523abf371a458b, 0x61910fcb6ff57c80, ALL),
    ("es", "a \u{1}V Ok", "'a\n'ok", "ˈa\nˈok", 29274, 0xcf411a2b18c88db3, 0xc8eb3f8eefcdf720, X | IPA),
    ("es", "a \u{1}V . Ok", "'a\n'ok", "ˈa\nˈok", 29274, 0xcf411a2b18c88db3, 0x6c719f877e5f0bdc, X | IPA),
    ("es", "~ . ok", "(en)t'Ild@(es) 'ok", "(en)tˈɪldə(es) ˈok", 38524, 0x5c5a460ae9115924, 0xd1cca721086aa93f, ALL),
    ("es", "% . ok", "porTj'Ento 'ok", "poɾθjˈɛnto ˈok", 50716, 0x69b45531245d4945, 0xb4393571a35995c1, ALL),
    ("es", "... . ok", "'ok", "ˈok", 23898, 0xfe3df99a24d7f70c, 0xdfe8d6c32466df4c, ALL),
    ("es", "~ .) Ok", "(en)t'Ild@(es)_:_: 'ok", "(en)tˈɪldə(es) ˈok", 43372, 0xb7aca037ec8dc864, 0x4ddcd8a3d00e0543, ALL),
    ("es", "-. Ok", "'ok", "ˈok", 23898, 0xfe3df99a24d7f70c, 0x3586a22b7717549c, ALL),
    ("es", "~\u{ad}. Ok", "(en)t'Ild@(es) 'ok", "(en)tˈɪldə(es) ˈok", 38524, 0x5c5a460ae9115924, 0xd1cca721086aa93f, X | IPA | WAV),
    ("es", "~\u{200d}. Ok", "(en)t'Ild@(es) 'ok", "(en)tˈɪldə(es) ˈok", 38524, 0x5c5a460ae9115924, 0x9470708dda12bcc2, ALL),
    ("es", "~\u{f0b}. Ok", "(en)t'Ild@(es) 'ok", "(en)tˈɪldə(es) ˈok", 38524, 0x5c5a460ae9115924, 0xd1cca721086aa93f, ALL),
    ("es", "+ . - . =", "m'as_:_: iQw'al", "mˈas iɣwˈal", 50742, 0xa3eb0e26c144415c, 0xee8db36fab099706, ALL),
    ("es", "a - b", "a_:_: b'e", "a bˈe", 31144, 0x7ab74c57a15e1941, 0xa7aee23dd6a1cd54, ALL),
    ("es", "~ . - . ~", "(en)t'Ild@(es)_:_: (en)t'Ild@(es)", "(en)tˈɪldə(es) (en)tˈɪldə(es)", 50426, 0xdfb08ff7ca64c047, 0xa5ca647fa26dc53e, ALL),
    ("ru", "a \u{1}V Ok", "(en)'eI(ru)\n(en),oUk'eI(ru)", "(en)ˈeɪ(ru)\n(en)ˌəʊkˈeɪ(ru)", 42152, 0x25ee6e4a5a7909ba, 0x36af98b3b6c71514, X | IPA),
    ("ru", "a \u{1}V . Ok", "(en)'eI(ru)\n(en),oUk'eI(ru)", "(en)ˈeɪ(ru)\n(en)ˌəʊkˈeɪ(ru)", 42152, 0x25ee6e4a5a7909ba, 0x68ec6d6e967ef380, X | IPA),
    ("ru", "~ . ok", "t;'ilda# (en),oUk'eI(ru)", "tʲˈiɭda (en)ˌəʊkˈeɪ(ru)", 43260, 0xafe054936dbab8ce, 0x1eb53456eccde400, ALL),
    ("ru", "% . ok", "p@-rV#ts'Ent (en),oUk'eI(ru)", "prʌtsˈɛnt (en)ˌəʊkˈeɪ(ru)", 56038, 0x3fcd36396cd5ac98, 0x18ea43c67eb93f67, ALL),
    ("ru", "... . ok", "(en),oUk'eI(ru)", "(en)ˌəʊkˈeɪ(ru)", 33110, 0x094002215eeb8f3f, 0x900948f6c9f8c1c4, ALL),
    ("ru", "~ .) Ok", "t;'ilda#_:_: (en),oUk'eI(ru)", "tʲˈiɭda (en)ˌəʊkˈeɪ(ru)", 48108, 0x5e8a623cafe8bfce, 0x6447393165e587cc, ALL),
    ("ru", "-. Ok", "(en),oUk'eI(ru)", "(en)ˌəʊkˈeɪ(ru)", 33110, 0x094002215eeb8f3f, 0x134c92d777db3bd4, ALL),
    ("ru", "~\u{ad}. Ok", "t;'ilda# (en),oUk'eI(ru)", "tʲˈiɭda (en)ˌəʊkˈeɪ(ru)", 43260, 0xafe054936dbab8ce, 0x1eb53456eccde400, X | IPA | WAV),
    ("ru", "~\u{200d}. Ok", "t;'ilda# (en),oUk'eI(ru)", "tʲˈiɭda (en)ˌəʊkˈeɪ(ru)", 43260, 0xafe054936dbab8ce, 0xfcd7997560c3b219, ALL),
    ("ru", "~\u{f0b}. Ok", "t;'ilda# (en),oUk'eI(ru)", "tʲˈiɭda (en)ˌəʊkˈeɪ(ru)", 43260, 0xafe054936dbab8ce, 0x1eb53456eccde400, ALL),
    ("ru", "~ .- Ok", "t;'ilda# (en),oUk'eI(ru)", "tʲˈiɭda (en)ˌəʊkˈeɪ(ru)", 43260, 0xafe054936dbab8ce, 0xb33ad184875b4952, ALL),
    ("ru", "+ . - . =", "pl;'u\"s_:_: @-ravn'o", "pɭʲˈu\"s ravnˈo", 48600, 0xb97f468fc1b007c3, 0x4aba57254bac51cf, ALL),
    ("ru", "a - b", "(en)'eI(ru)_:_: (en)b'i:(ru)", "(en)ˈeɪ(ru) (en)bˈiː(ru)", 40206, 0xb02bb553def06154, 0x2233da481ca7bf63, ALL),
    ("ru", "~ . - . ~", "t;'ilda#_:_: t;'ilda#", "tʲˈiɭda tʲˈiɭda", 39096, 0x6486c27216a6c35d, 0x65cffea191200aba, ALL),
    ("pl", "~ . ok", "t'ylda Ok'OwO", "tˈɨlda ɔkˈɔwɔ", 49828, 0x7d5aa230b108ca59, 0x0c710690b8417c4a, ALL),
    ("pl", "% . ok", "pR'OtsEnt Ok'OwO", "prˈɔtsɛnt ɔkˈɔwɔ", 59608, 0x0327a29e4f12b7a3, 0x587320208ed8c65a, ALL),
    ("pl", "... . ok", "Ok'OwO", "ɔkˈɔwɔ", 33620, 0x0dde06a9d67f555a, 0x31585bed0a9000d7, ALL),
    ("pl", "~ .) Ok", "t'ylda_:_: Ok'OwO", "tˈɨlda ɔkˈɔwɔ", 56328, 0x78e3d8820443c287, 0xb97ec6de071316ef, ALL),
    ("pl", "-. Ok", "Ok'OwO", "ɔkˈɔwɔ", 33620, 0x0dde06a9d67f555a, 0xd148e135c3da2727, ALL),
    ("pl", "~\u{ad}. Ok", "t'ylda Ok'OwO", "tˈɨlda ɔkˈɔwɔ", 49828, 0x7d5aa230b108ca59, 0x0c710690b8417c4a, X | IPA | WAV),
    ("pl", "~\u{f0b}. Ok", "t'ylda Ok'OwO", "tˈɨlda ɔkˈɔwɔ", 49828, 0x7d5aa230b108ca59, 0x0c710690b8417c4a, ALL),
    ("pl", "+ . - . =", "pl'us_:_: R'uvna S;,E", "plˈus rˈuvna ɕˌɛ", 58874, 0xf646e49001b96bf4, 0x32726461974769a1, ALL),
    ("pl", "a - b", "_!a__:_: b'E", "a bˈɛ", 33842, 0xebc2b5b8a7027e37, 0x3d30e7b057ce68ff, ALL),
    ("pl", "~ . - . ~", "t'ylda_:_: t'ylda", "tˈɨlda tˈɨlda", 56150, 0x09658bd3ebbfd823, 0x5abd8d1f2b8e0bed, ALL),
    ("ja", "~ . ok", "(en)t'Ild@ _:'oU _:k'eI(ja)", "(en)tˈɪldə ˈəʊ kˈeɪ(ja)", 57940, 0x598cc1b89bced23e, 0x8f7c9d7a6520af5c, X | IPA | WAV),
    ("ja", "% . ok", "(en)p3s'Ent _:'oU _:k'eI(ja)", "(en)pəsˈɛnt ˈəʊ kˈeɪ(ja)", 61706, 0xce3d0cafb88c4c8e, 0xc5bd3174afa10f15, X | IPA | WAV),
    ("ja", "... . ok", "_:(en)'oU _:k'eI(ja)", "(en)ˈəʊ kˈeɪ(ja)", 41622, 0x1851b1cb8d9ccc44, 0x7d72188084838ed4, X | IPA | WAV),
    ("ja", "-. Ok", "_:(en)'oU _:k'eI(ja)", "(en)ˈəʊ kˈeɪ(ja)", 41622, 0x1851b1cb8d9ccc44, 0xd27469c8bc7a765d, X | IPA | WAV),
    ("ja", "~ .- Ok", "(en)t'Ild@ _:'oU _:k'eI(ja)", "(en)tˈɪldə ˈəʊ kˈeɪ(ja)", 57940, 0x598cc1b89bced23e, 0x805f5e279f3f34cf, X | IPA | WAV),
    ("ja", "+ . - . =", "(en)pl'Vs(ja)_:_: (en)'i:kw@Lz(ja)", "(en)plˈʌs(ja) (en)ˈiːkwəlz(ja)", 55748, 0x154dcfd7d618b813, 0x1c4ef1fd70dec688, ALL),
    ("ja", "a - b", "_:(en)'eI(ja)_:_: _:(en)b'i:(ja)", "(en)ˈeɪ(ja) (en)bˈiː(ja)", 44702, 0xcad9ec966c22d461, 0xcc37279c6ac5f44b, X | IPA | WAV),
    ("ja", "~ . - . ~", "(en)t'Ild@(ja)_:_: (en)t'Ild@(ja)", "(en)tˈɪldə(ja) (en)tˈɪldə(ja)", 54302, 0xd899a783d28ed42d, 0x5eac3d162ef4a5e5, ALL),
    ("cmn", "a \u{1}V Ok", "(en)'eI55(cmn)_|\n(en),oU55k'eI55(cmn)_|", "(en)ˈeɪ5(cmn)\n(en)ˌəʊəʊkˈeɪ5(cmn)", 36822, 0x6789f440d2737341, 0xddc9e7dddfd43e46, X | IPA),
    ("cmn", "~ . ok", "(en)t'I55ld@55_| ,oU55k'eI55(cmn)_|", "(en)tˈɪ5ldə5 ˌəʊəʊkˈeɪ5(cmn)", 44484, 0x10fa63996902adbc, 0x484f139dca29f01c, X | IPA),
    ("cmn", "~ .- Ok", "(en)t'I55ld@55_| ,oU55k'eI55(cmn)_|", "(en)tˈɪ5ldə5 ˌəʊəʊkˈeɪ5(cmn)", 44484, 0x10fa63996902adbc, 0x89e727808d00f12a, X | IPA),
    ("cmn", "+ . - . =", "(en)pl'V55s(cmn)_:_:_| (en)'i:55kw@L55z(cmn)_|", "(en)plˈʌ5s(cmn) (en)ˈiː5kwələlz(cmn)", 53632, 0x605a38d063cec54d, 0x310e3f1d5f382394, X | IPA),
    ("ko", "a \u{1}V Ok", "(en)'eI(ko)\n(en),oUk'eI(ko)", "(en)ˈeɪ(ko)\n(en)ˌəʊkˈeɪ(ko)", 42222, 0x3d4a7f9f21262962, 0x283b3538e15fef9a, X | IPA),
    ("ko", "% . ok", "(en)p3s'Ent ,oUk'eI(ko)", "(en)pəsˈɛnt ˌəʊkˈeɪ(ko)", 52490, 0xa648e30549617b38, 0x654437a26181a7be, ALL),
    ("ko", "... . ok", "(en),oUk'eI(ko)", "(en)ˌəʊkˈeɪ(ko)", 33104, 0xdbc4a085c127b575, 0x122be99d07be0eec, ALL),
    ("ko", "~ .) Ok", "(en)t'Ild@(ko)_:_: (en),oUk'eI(ko)", "(en)tˈɪldə(ko) (en)ˌəʊkˈeɪ(ko)", 54956, 0xce0fa0cc1da609fd, 0xe1f8ae2a622932bd, ALL),
    ("ko", "+ . - . =", "(en)pl'Vs(ko)_:_: (en)'i:kw@Lz(ko)", "(en)plˈʌs(ko) (en)ˈiːkwəlz(ko)", 55594, 0xb63e90609b8c4d24, 0x61c306df9ae684e4, ALL),
    ("ko", "a - b", "(en)'eI(ko)_:_: (en)b'i:(ko)", "(en)ˈeɪ(ko) (en)bˈiː(ko)", 40042, 0x4297633d02a7439f, 0xca581e7b38a97f84, ALL),
    ("ca", "~ . ok", "t'idl^@ 'Ok", "tˈidʎə ˈɔk", 37534, 0xd1c8ed1a36c46d67, 0xae3bcb4257838e8b, ALL),
    ("ca", "% . ok", "p@Rs'en 'Ok", "pərsˈen ˈɔk", 40846, 0x815a68b90e6836f8, 0xe42a039e162ffbe9, ALL),
    ("ca", "... . ok", "'Ok", "ˈɔk", 23452, 0xa4d32827e77ef26d, 0xd5e7d50a2c828859, ALL),
    ("ca", "~ .) Ok", "t'idl^@_:_: 'Ok", "tˈidʎə ˈɔk", 43660, 0xdbd12889525c2f12, 0xfeb6193767312331, ALL),
    ("ca", "-. Ok", "'Ok", "ˈɔk", 23452, 0xa4d32827e77ef26d, 0x77cdd619032d58a9, ALL),
    ("ca", "~ .- Ok", "t'idl^@ 'Ok", "tˈidʎə ˈɔk", 37534, 0xd1c8ed1a36c46d67, 0x72282014bb85a899, ALL),
    ("ca", "+ . - . =", "m'es_:_: iQw'al", "mˈes iɣwˈal", 48978, 0x7cb8037eb39929f2, 0xb9f3db58cf542285, ALL),
    ("ca", "a - b", "@_:_: b'e", "ə bˈe", 30184, 0xa8f27cb0c75ca06b, 0x5de7c0e787375482, ALL),
    ("ca", "~ . - . ~", "t'idl^@_:_: t'idl^@", "tˈidʎə tˈidʎə", 50588, 0x69e7fc91493d7166, 0xef6563060b585fd4, ALL),
];

fn differences(row: &Row) -> Vec<String> {
    let &(voice, input, x, ipa, bytes, hash, events, fields) = row;
    let mut checks: Vec<(&str, bool)> = Vec::new();
    if fields & X != 0 {
        checks.push(("-x", text(voice, "-x", input) == x));
    }
    if fields & IPA != 0 {
        checks.push(("--ipa", text(voice, "--ipa", input) == ipa));
    }
    if fields & WAV != 0 {
        let audio = pcm(voice, input);
        checks.push(("wav", (audio.len(), fnv1a(&audio)) == (bytes, hash)));
    }
    if fields & EVENTS != 0 {
        checks.push(("events", transcript_hash(voice, input) == events));
    }
    checks.into_iter().filter(|&(_, same)| !same).map(|(field, _)| format!("{voice} {field} {input:?}")).collect()
}

#[test]
fn clause_rule_edges_match_c() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = ROWS.iter().flat_map(differences).collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
