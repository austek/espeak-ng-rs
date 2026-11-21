// tests/any_alnum_voice_clause.rs
//
// C ends the clause at a bare `\x01V` with `CLAUSE_VOICE`: no pause, no intonation, no sentence.
// A mark before any letter or digit that no blank follows (`?~`) stays text and the clause ends
// at the end of the text, and a spaced dash after a blanked `.` still pauses.  Expected values
// are what C espeak-ng master (f0fb11a8) produces: `-q -x`, `-q --ipa`, the PCM of `-w` (byte
// count and FNV-1a 64 hash) and the word/sentence/end events (FNV-1a 64 hash of the transcript
// lines `ev <pos> <len> <ms> <sample> <type> num=<n>` and `samples=<n>`).
//
// Each row's last field says which of the four this port reproduces.  The others differ from C
// for reasons outside the rule: the word split of a switched English word in Japanese, Mandarin
// clause pauses, and one-letter words in Catalan and Polish.

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
    let dir = std::env::temp_dir().join(format!("any_alnum_voice_{}", std::process::id()));
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
    ("en", "a \u{1}V Ok", "'eI\n,oUk'eI", "ˈeɪ\nˌəʊkˈeɪ", 40376, 0xc3082326fc1a0654, 0x95511cc6001f4fb4, ALL),
    ("en", "Hi \u{1}V ok", "h'aI\n,oUk'eI", "hˈaɪ\nˌəʊkˈeɪ", 44002, 0x0c888791f2a7f8c9, 0xdf880d8117bac708, ALL),
    ("en", ". \u{1}V . \u{1}V . a", "'eI", "ˈeɪ", 23682, 0x8b111589865601bc, 0xddddbfb6e1a04471, ALL),
    ("en", "~ \u{1}V ~ . a", "t'Ild@\nt'Ild@r- 'eI", "tˈɪldə\ntˈɪldəɹ ˈeɪ", 56650, 0x550ea051d5d5ab4d, 0x95adf4a8c769c4c4, ALL),
    ("en", "Hi. \u{1}V ok", "h'aI\n\n,oUk'eI", "hˈaɪ\n\nˌəʊkˈeɪ", 59890, 0xbc5c81c5a9f8d4ff, 0x6459c963c9ddcfc6, ALL),
    ("en", "Hi. . ?~", "h'aI\nt'Ild@", "hˈaɪ\ntˈɪldə", 59904, 0x86f5a73f61ddee64, 0xd8e817087a4ec5bc, X | IPA | WAV),
    ("en", "Mr. . ?~", "m'Ist3\nt'Ild@", "mˈɪstə\ntˈɪldə", 47598, 0xb0dca13a06d72223, 0x08dd1135a195d353, X | IPA | WAV),
    ("en", "Hi. ?~", "h'aI\nt'Ild@", "hˈaɪ\ntˈɪldə", 59904, 0x86f5a73f61ddee64, 0x8e583d4a56f27256, X | IPA | WAV),
    ("de", "a \u{1}V Ok", "_!'A:\n_|o:k'e:", "ˈɑː\noːkˈeː", 40676, 0xab347ce91780caf2, 0xabb51115f6d3b496, ALL),
    ("de", "~ \u{1}V . ok", "t'Ild@\n_|o:k'e:", "tˈɪldə\noːkˈeː", 46782, 0x6f1751923ac9e0b6, 0xf2b002d13a9763ea, ALL),
    ("de", "Hi \u{1}V ok", "h'aI\n_|o:k'e:", "hˈaɪ\noːkˈeː", 42124, 0xdb5030aa02840446, 0xabdb8301cd5f4cf2, ALL),
    ("de", ". \u{1}V . \u{1}V . a", "_!'A:", "ˈɑː", 26302, 0x6ab3ac31ef5d176b, 0x95f5eb51e53a7392, ALL),
    ("de", "~ \u{1}V ~ . a", "t'Ild@\nt'Ild@ _!'A:", "tˈɪldə\ntˈɪldə ˈɑː", 58986, 0x8249c26edefcf2fe, 0xbb10e0c5473fc0b3, ALL),
    ("de", "Hi. \u{1}V ok", "h'aI\n\n_|o:k'e:", "hˈaɪ\n\noːkˈeː", 56556, 0x11803311bad76e16, 0x188d69c836c7c276, ALL),
    ("de", "Hi. . ?~", "h'aI\nt'Ild@", "hˈaɪ\ntˈɪldə", 58398, 0x3ef3dd6ebea3f554, 0x0bb72d3e363dbb9e, X | IPA | WAV),
    ("de", "Mr. . ?~", "_!,Em'Er\nt'Ild@", "ˌɛmˈɛɾ\ntˈɪldə", 59386, 0xa43d869d36100872, 0x7239dde86503f2ef, X | IPA | WAV),
    ("de", "Hi. ?~", "h'aI\nt'Ild@", "hˈaɪ\ntˈɪldə", 58398, 0x3ef3dd6ebea3f554, 0x4f18f0ccdf6c5f10, X | IPA | WAV),
    ("fr", "a \u{1}V Ok", "'a\nok'e", "ˈa\nokˈe", 31068, 0x0cf682cc9b1f3d61, 0x1f73b6f735e9417c, ALL),
    ("fr", "~ \u{1}V . ok", "tild'e\nok'e", "tildˈe\nokˈe", 42108, 0x4efd42c3c8cd6483, 0x36a21101237fc231, ALL),
    ("fr", "Hi \u{1}V ok", "_|'i\nok'e", "ˈi\nokˈe", 30690, 0x90a9b4166e1484c6, 0x19524f125d43ad91, IPA),
    ("fr", ". \u{1}V . \u{1}V . a", "'a", "ˈa", 20258, 0x05421b67b17bb38c, 0xed82de48779636b0, ALL),
    ("fr", "~ \u{1}V ~ . a", "tild'e\ntild'e 'a", "tildˈe\ntildˈe ˈa", 50246, 0xfa3c4e7911264a16, 0x79175857feed604e, ALL),
    ("fr", "Hi. \u{1}V ok", "_|'i\n\nok'e", "ˈi\n\nokˈe", 46116, 0x77fdd117ae9cdb88, 0x05789fdd9b63b4bb, IPA),
    ("fr", "Hi. . ?~", "_|'i\ntild'e", "ˈi\ntildˈe", 50038, 0x60e93ac094623c6f, 0x36557c138fb1e1c0, X | IPA | WAV),
    ("fr", "Mr. . ?~", "m@sj'Y\ntild'e", "məsjˈø\ntildˈe", 62760, 0x4ed63a1348f516e5, 0xf51ffa30259a4836, X | IPA | WAV),
    ("fr", "Hi. ?~", "_|'i\ntild'e", "ˈi\ntildˈe", 50038, 0x60e93ac094623c6f, 0xdd94d07b4b790b5e, X | IPA | WAV),
    ("es", "a \u{1}V Ok", "'a\n'ok", "ˈa\nˈok", 29274, 0xcf411a2b18c88db3, 0xc8eb3f8eefcdf720, ALL),
    ("es", "~ \u{1}V . ok", "(en)t'Ild@(es)\n'ok", "(en)tˈɪldə(es)\nˈok", 39308, 0xa15fd418883a6bc4, 0x79e80ed0d4c7495d, ALL),
    ("es", "Hi \u{1}V ok", "'i\n'ok", "ˈi\nˈok", 29274, 0xabbd9a6bbd2bc66f, 0xbe7dd3bffeb11e9a, IPA),
    ("es", ". \u{1}V . \u{1}V . a", "'a", "ˈa", 21268, 0x905969953d98c95e, 0xf932396ae557edb4, ALL),
    ("es", "~ \u{1}V ~ . a", "(en)t'Ild@(es)\n(en)t'Ild@(es) 'a", "(en)tˈɪldə(es)\n(en)tˈɪldə(es) ˈa", 50740, 0x6aed59fdb7fa7aea, 0xd542bbdc8de68474, ALL),
    ("es", "Hi. \u{1}V ok", "'i\n\n'ok", "ˈi\n\nˈok", 45178, 0xf4ca7d4c29cc9c2a, 0x6d05de6ac163b570, IPA),
    ("es", "Hi. . ?~", "'i\n(en)t'Ild@(es)", "ˈi\n(en)tˈɪldə(es)", 51634, 0x806b7b62e48f9991, 0x5bf369122be5b865, X | IPA | WAV),
    ("es", "Mr. . ?~", ",eme'ER2e\n(en)t'Ild@(es)", "ˌemeˈɛre\n(en)tˈɪldə(es)", 62858, 0xbd7ebee9f90ac63b, 0xa8141c2d6d25de03, X | IPA | WAV),
    ("es", "Hi. ?~", "'i\n(en)t'Ild@(es)", "ˈi\n(en)tˈɪldə(es)", 51634, 0x806b7b62e48f9991, 0xa9b0ef7caa080673, X | IPA | WAV),
    ("ru", "a \u{1}V Ok", "(en)'eI(ru)\n(en),oUk'eI(ru)", "(en)ˈeɪ(ru)\n(en)ˌəʊkˈeɪ(ru)", 42152, 0x25ee6e4a5a7909ba, 0x36af98b3b6c71514, ALL),
    ("ru", "~ \u{1}V . ok", "t;'ilda#\n(en),oUk'eI(ru)", "tʲˈiɭda\n(en)ˌəʊkˈeɪ(ru)", 43704, 0x5e1df97050d1e0d0, 0x706f3ad57fb5928c, ALL),
    ("ru", "Hi \u{1}V ok", "(en)h'aI(ru)\n(en),oUk'eI(ru)", "(en)hˈaɪ(ru)\n(en)ˌəʊkˈeɪ(ru)", 45658, 0xcc7da679f0c37b74, 0xb434a46bd8cb8c22, ALL),
    ("ru", ". \u{1}V . \u{1}V . a", "(en)'eI(ru)", "(en)ˈeɪ(ru)", 24270, 0x1acf5dc8c0135aee, 0xdb7e3a27d5ad4a10, ALL),
    ("ru", "~ \u{1}V ~ . a", "t;'ilda#\nt;'ilda# (en)'eI(ru)", "tʲˈiɭda\ntʲˈiɭda (en)ˈeɪ(ru)", 45036, 0x92a1335fafae40ba, 0x042857aeb915f8a9, ALL),
    ("ru", "Hi. \u{1}V ok", "(en)h'aI(ru)\n\n(en),oUk'eI(ru)", "(en)hˈaɪ(ru)\n\n(en)ˌəʊkˈeɪ(ru)", 61710, 0xaf71813c7e31dc71, 0x9934b8e9d229a09f, ALL),
    ("ru", "Hi. . ?~", "(en)h'aI(ru)\nt;'ilda#", "(en)hˈaɪ(ru)\ntʲˈiɭda", 52668, 0x89dd189d063eca3f, 0x48ac729d482e5a12, X | IPA | WAV),
    ("ru", "Mr. . ?~", "(en)m'Ist3(ru)\nt;'ilda#", "(en)mˈɪstə(ru)\ntʲˈiɭda", 39920, 0x9b0b2d3827d82595, 0xb2ae07222a194890, X | IPA),
    ("ru", "Hi. ?~", "(en)h'aI(ru)\nt;'ilda#", "(en)hˈaɪ(ru)\ntʲˈiɭda", 52668, 0x89dd189d063eca3f, 0x217fc11ef0131264, X | IPA | WAV),
    ("pl", "a \u{1}V Ok", "a_\nOk'OwO", "a\nɔkˈɔwɔ", 39444, 0x55c47312415fb960, 0xecbcc7e771617ac0, WAV | EVENTS),
    ("pl", "~ \u{1}V . ok", "t'ylda\nOk'OwO", "tˈɨlda\nɔkˈɔwɔ", 52022, 0x5a19835e606a3849, 0x49d34d935f500641, ALL),
    ("pl", "Hi \u{1}V ok", "C'i\nOk'OwO", "çˈi\nɔkˈɔwɔ", 42954, 0x42600b3ba31bd2dc, 0x06328f5338c7ee51, IPA),
    ("pl", ". \u{1}V . \u{1}V . a", "'a_", "ˈa", 21802, 0x84d4ad905a8e8b32, 0xe4e5e37d5f91b411, ALL),
    ("pl", "~ \u{1}V ~ . a", "t'ylda\nt'ylda a_", "tˈɨlda\ntˈɨlda a", 55906, 0x965bef5c7e3842de, 0x24c0156207dd299f, ALL),
    ("pl", "Hi. \u{1}V ok", "C'i\n\nOk'OwO", "çˈi\n\nɔkˈɔwɔ", 57336, 0xe12b4b4b9dc4ebad, 0xdd7e345f737e6a39, IPA),
    ("uk", "a \u{1}V Ok", "(en)'eI(uk)\n(en),oUk'eI(uk)", "(en)ˈeɪ(uk)\n(en)ˌəʊkˈeɪ(uk)", 56950, 0x968d8ad3a75b7539, 0x564593cc93bf1704, ALL),
    ("uk", "~ \u{1}V . ok", "t'Il^da\n(en),oUk'eI(uk)", "tˈɪʎda\n(en)ˌəʊkˈeɪ(uk)", 67994, 0xa404a1eb6258c9e2, 0xf65644b4d1e45838, ALL),
    ("uk", "Hi \u{1}V ok", "(en)h'aI(uk)\n(en),oUk'eI(uk)", "(en)hˈaɪ(uk)\n(en)ˌəʊkˈeɪ(uk)", 61326, 0x1d002face2d28cac, 0x575db826508fc20e, ALL),
    ("uk", ". \u{1}V . \u{1}V . a", "(en)'eI(uk)", "(en)ˈeɪ(uk)", 33802, 0x18ae6e4325ed9a03, 0x7897f6000369639d, ALL),
    ("uk", "~ \u{1}V ~ . a", "t'Il^da\nt'Il^da (en)'eI(uk)", "tˈɪʎda\ntˈɪʎda (en)ˈeɪ(uk)", 80264, 0x38a8f0dba3c31f57, 0x33441f6ee17bce9f, ALL),
    ("uk", "Hi. \u{1}V ok", "(en)h'aI(uk)\n\n(en),oUk'eI(uk)", "(en)hˈaɪ(uk)\n\n(en)ˌəʊkˈeɪ(uk)", 83042, 0xab6d32df30e85896, 0xe36b3f55a75cc31d, ALL),
    ("uk", "Hi. . ?~", "(en)h'aI(uk)\nt'Il^da", "(en)hˈaɪ(uk)\ntˈɪʎda", 83430, 0xdaa3cfbc5db3ee53, 0x56f9300cabaaedfe, X | IPA | WAV),
    ("uk", "Mr. . ?~", "(en)m'Ist3(uk)\nt'Il^da", "(en)mˈɪstə(uk)\ntˈɪʎda", 64964, 0x64d13aaeb81c991f, 0x209346909e4af28b, X | IPA),
    ("uk", "Hi. ?~", "(en)h'aI(uk)\nt'Il^da", "(en)hˈaɪ(uk)\ntˈɪʎda", 83430, 0xdaa3cfbc5db3ee53, 0xa0f71c330c15722c, X | IPA | WAV),
    ("uk", "--- . --- so", "m'i:nus_:_: m'i:nus (en)s'oU(uk)", "mˈiːnus mˈiːnus (en)sˈəʊ(uk)", 88338, 0x73addbe0b1ec649f, 0xb961de38f419941a, ALL),
    ("el", "a \u{1}V Ok", "(en)'eI(el)\n(en),oUk'eI(el)", "(en)ˈeɪ(el)\n(en)ˌəʊkˈeɪ(el)", 40012, 0xff97aeb04fc298e0, 0x2948ddb938585f0d, ALL),
    ("el", "~ \u{1}V . ok", "(en)t'Ild@(el)\n(en),oUk'eI(el)", "(en)tˈɪldə(el)\n(en)ˌəʊkˈeɪ(el)", 48684, 0xf30a5a0c2ee6e752, 0xececbd4fac035d1a, ALL),
    ("el", "Hi \u{1}V ok", "(en)h'aI(el)\n(en),oUk'eI(el)", "(en)hˈaɪ(el)\n(en)ˌəʊkˈeɪ(el)", 43516, 0x110fee17a67829e7, 0x7961a5f6fcc69c4a, ALL),
    ("el", ". \u{1}V . \u{1}V . a", "(en)'eI(el)", "(en)ˈeɪ(el)", 23870, 0x2dbdea58ee673745, 0x3d4ff8f1c8c4c72e, ALL),
    ("el", "Hi. \u{1}V ok", "(en)h'aI(el)\n\n(en),oUk'eI(el)", "(en)hˈaɪ(el)\n\n(en)ˌəʊkˈeɪ(el)", 59132, 0x3ee981d4d842e259, 0x06d7f05a330dc489, ALL),
    ("el", "Hi. . ?~", "(en)h'aI(el)\n(en)t'Ild@(el)", "(en)hˈaɪ(el)\n(en)tˈɪldə(el)", 58398, 0xa058708223eb2fee, 0x658bb9e75acc3157, X | IPA | WAV),
    ("el", "Mr. . ?~", "(en)m'Ist3(el)\n(en)t'Ild@(el)", "(en)mˈɪstə(el)\n(en)tˈɪldə(el)", 46828, 0xb031efb443562209, 0x1414b26c4074500c, X | IPA),
    ("el", "Hi. ?~", "(en)h'aI(el)\n(en)t'Ild@(el)", "(en)hˈaɪ(el)\n(en)tˈɪldə(el)", 58398, 0xa058708223eb2fee, 0x5e4ce5d821434a21, X | IPA | WAV),
    ("el", "--- . --- so", "m'i;on_:_: m'i;on (en)s'oU(el)", "mˈion mˈion (en)sˈəʊ(el)", 58514, 0x1aca77d080e867b0, 0x82aa2b7f31fcf15c, ALL),
    ("pt", "a \u{1}V Ok", "'a\nOk'eI", "ˈa\nɔkˈeɪ", 35808, 0xe716a1ccc647775f, 0x86bdc31ab993fa81, ALL),
    ("pt", "~ \u{1}V . ok", "Ok'eI", "ɔkˈeɪ", 29632, 0x595a2075f9993209, 0x42de12c298f6f18b, ALL),
    ("pt", "Hi \u{1}V ok", "'i\nOk'eI", "ˈi\nɔkˈeɪ", 35416, 0xf13b283b98ed85b2, 0x31cf62a344e61852, IPA),
    ("pt", ". \u{1}V . \u{1}V . a", "'a", "ˈa", 21188, 0x476446de76954c2d, 0xd67ab90214d5f0ea, ALL),
    ("pt", "~ \u{1}V ~ . a", "'a", "ˈa", 21188, 0x476446de76954c2d, 0xb2df1f853ad3fce9, ALL),
    ("pt", "Hi. \u{1}V ok", "'i\n\nOk'eI", "ˈi\n\nɔkˈeɪ", 50872, 0xfa0ebf8cd89223bf, 0xef70027fc4e2b8ff, IPA),
    ("pt", "Hi. . ?~", "'i", "ˈi", 34516, 0xd5246aeb648f9079, 0x73371dc8f5b3e1d9, ALL),
    ("pt", "Mr. . ?~", "m'ister", "mˈister", 29454, 0x0e99a3d1ad9e6b2d, 0xa6e2e4a0ff83fcc0, ALL),
    ("pt", "Hi. ?~", "'i", "ˈi", 34516, 0xd5246aeb648f9079, 0xaa6137303f7ddcdb, ALL),
    ("tr", "a \u{1}V Ok", "'a\n'Ok", "ˈa\nˈɔk", 32134, 0x557deb885039046e, 0x87b7ee4877eaa17a, ALL),
    ("tr", "~ \u{1}V . ok", "'Ok", "ˈɔk", 24466, 0xee5aa26609c03b1a, 0x13b6eb6cf0c53263, ALL),
    ("tr", "Hi \u{1}V ok", "h'I\n'Ok", "hˈɪ\nˈɔk", 34640, 0xd190389c749386ec, 0x2aa88b0b0254cf6f, ALL),
    ("tr", ". \u{1}V . \u{1}V . a", "'a", "ˈa", 22718, 0x5b87ae9694b2393a, 0x53d245e2afa26071, ALL),
    ("tr", "~ \u{1}V ~ . a", "'a", "ˈa", 22718, 0x5b87ae9694b2393a, 0x0513c4fa20e6b9f2, ALL),
    ("tr", "Hi. \u{1}V ok", "h'I\n\n'Ok", "hˈɪ\n\nˈɔk", 49598, 0x0df22a4c590cbc7a, 0xc377f81e22399e5d, ALL),
    ("tr", "Hi. . ?~", "h'I", "hˈɪ", 38408, 0x5768dd298ab4470f, 0x6b211dd753411235, ALL),
    ("tr", "Mr. . ?~", "m,E*'E", "mˌɛɾˈɛ", 45304, 0x715a7ba6767d904f, 0xc67f53df11eccba9, ALL),
    ("tr", "Hi. ?~", "h'I", "hˈɪ", 38408, 0x5768dd298ab4470f, 0x47d0339a6b94927b, ALL),
    ("ja", "~ \u{1}V . ok", "(en)t'Ild@(ja)\n_:(en)'oU _:k'eI(ja)", "(en)tˈɪldə(ja)\n(en)ˈəʊ kˈeɪ(ja)", 59084, 0xacf727533ffd7e10, 0x335b5f6f4b760fe0, X | IPA | WAV),
    ("ja", ". \u{1}V . \u{1}V . a", "_:(en)'eI(ja)", "(en)ˈeɪ(ja)", 26920, 0xef5d7ce23e2bc0d8, 0xf5bed3148535ab27, ALL),
    ("ja", "~ \u{1}V ~ . a", "(en)t'Ild@(ja)\n(en)t'Ild@ _:'eI(ja)", "(en)tˈɪldə(ja)\n(en)tˈɪldə ˈeɪ(ja)", 60862, 0x406d4752a6c25fe6, 0x77ab224a6f22963a, ALL),
    ("ja", "Hi. . ?~", "_:(en)'eItS _:'aI(ja)\n(en)t'Ild@(ja)", "(en)ˈeɪtʃ ˈaɪ(ja)\n(en)tˈɪldə(ja)", 74888, 0x25c0b07d202b9b21, 0x09b59d52ea36f37d, X | IPA | WAV),
    ("ja", "Hi. ?~", "_:(en)'eItS _:'aI(ja)\n(en)t'Ild@(ja)", "(en)ˈeɪtʃ ˈaɪ(ja)\n(en)tˈɪldə(ja)", 74888, 0x25c0b07d202b9b21, 0xe4b3e7ef2741d92b, X | IPA | WAV),
    ("cmn", "a \u{1}V Ok", "(en)'eI55(cmn)_|\n(en),oU55k'eI55(cmn)_|", "(en)ˈeɪ5(cmn)\n(en)ˌəʊəʊkˈeɪ5(cmn)", 36822, 0x6789f440d2737341, 0xddc9e7dddfd43e46, X | IPA),
    ("cmn", "~ \u{1}V . ok", "(en)t'I55ld@55(cmn)_|\n(en),oU55k'eI55(cmn)_|", "(en)tˈɪ5ldə5(cmn)\n(en)ˌəʊəʊkˈeɪ5(cmn)", 46440, 0x9ef89dae3e165d08, 0xd112f3201691ace8, X | IPA),
    ("cmn", "Hi \u{1}V ok", "(en)h'aI55(cmn)_|\n(en),oU55k'eI55(cmn)_|", "(en)hˈaɪ5(cmn)\n(en)ˌəʊəʊkˈeɪ5(cmn)", 39554, 0xcdebefedb692539b, 0x4542782a7f853f0c, X | IPA),
    ("cmn", ". \u{1}V . \u{1}V . a", "(en)'eI55(cmn)_|", "(en)ˈeɪ5(cmn)", 20676, 0xd5c1f214a376cd44, 0x11e7fc4ad95ca978, X | IPA),
    ("cmn", "~ \u{1}V ~ . a", "(en)t'I55ld@55(cmn)_|\n(en)t'I55ld@55_| 'eI55(cmn)_|", "(en)tˈɪ5ldə5(cmn)\n(en)tˈɪ5ldə5 ˈeɪ5(cmn)", 52786, 0xc2ead8a737f243f2, 0x0dbfa1dd5b01e274, X | IPA),
    ("cmn", "Hi. \u{1}V ok", "(en)h'aI55(cmn)_|\n\n(en),oU55k'eI55(cmn)_|", "(en)hˈaɪ5(cmn)\n\n(en)ˌəʊəʊkˈeɪ5(cmn)", 52828, 0x8d182c2f311ae9eb, 0xa20a4a197b64adf3, X | IPA),
    ("cmn", "Hi. . ?~", "(en)h'aI55(cmn)_|\n(en)t'I55ld@55(cmn)_|", "(en)hˈaɪ5(cmn)\n(en)tˈɪ5ldə5(cmn)", 53728, 0xf518681ca6ff3125, 0xab54d3120704fea7, X | IPA),
    ("cmn", "Hi. ?~", "(en)h'aI55(cmn)_|\n(en)t'I55ld@55(cmn)_|", "(en)hˈaɪ5(cmn)\n(en)tˈɪ5ldə5(cmn)", 53728, 0xf518681ca6ff3125, 0x76be31a656d67bb1, X | IPA),
    ("ko", "a \u{1}V Ok", "(en)'eI(ko)\n(en),oUk'eI(ko)", "(en)ˈeɪ(ko)\n(en)ˌəʊkˈeɪ(ko)", 42222, 0x3d4a7f9f21262962, 0x283b3538e15fef9a, ALL),
    ("ko", "~ \u{1}V . ok", "(en)t'Ild@(ko)\n(en),oUk'eI(ko)", "(en)tˈɪldə(ko)\n(en)ˌəʊkˈeɪ(ko)", 50590, 0x1fd4ea433e966019, 0x0484ef937869498a, ALL),
    ("ko", "Hi \u{1}V ok", "(en)h'aI(ko)\n(en),oUk'eI(ko)", "(en)hˈaɪ(ko)\n(en)ˌəʊkˈeɪ(ko)", 45736, 0xd63f485d4441f419, 0x09d50925061c584c, ALL),
    ("ko", ". \u{1}V . \u{1}V . a", "(en)'eI(ko)", "(en)ˈeɪ(ko)", 24270, 0x5ffdd505d0ea692f, 0xdb7e3a27d5ad4a10, ALL),
    ("ko", "Hi. \u{1}V ok", "(en)h'aI(ko)\n\n(en),oUk'eI(ko)", "(en)hˈaɪ(ko)\n\n(en)ˌəʊkˈeɪ(ko)", 61698, 0xfd0fcacc815040be, 0x36771605e879ae4f, ALL),
    ("ko", "Hi. . ?~", "(en)h'aI(ko)\n(en)t'Ild@(ko)", "(en)hˈaɪ(ko)\n(en)tˈɪldə(ko)", 61166, 0x45c5cef9a486af72, 0xfe664ed3ef78e5d1, X | IPA | WAV),
    ("ko", "Mr. . ?~", "(en)m'Ist3(ko)\n(en)t'Ild@(ko)", "(en)mˈɪstə(ko)\n(en)tˈɪldə(ko)", 49302, 0xc34f1eb17ed5572b, 0x2e394b545da18870, X | IPA),
    ("ko", "Hi. ?~", "(en)h'aI(ko)\n(en)t'Ild@(ko)", "(en)hˈaɪ(ko)\n(en)tˈɪldə(ko)", 61166, 0x45c5cef9a486af72, 0x420c8c8198fa17f7, X | IPA | WAV),
    ("ca", "a \u{1}V Ok", "@\n'Ok", "ə\nˈɔk", 27096, 0xef1f9673b6a0e9a2, 0x338a7eddac7767fa, WAV | EVENTS),
    ("ca", "~ \u{1}V . ok", "t'idl^@\n'Ok", "tˈidʎə\nˈɔk", 39220, 0x0c01b86dc6602961, 0x40030c70272c49ae, ALL),
    ("ca", "Hi \u{1}V ok", "i\n'Ok", "i\nˈɔk", 27546, 0xe66fb462b7fef31b, 0x585774969393192d, WAV | EVENTS),
    ("ca", ". \u{1}V . \u{1}V . a", "'@", "ˈə", 19184, 0xaee63bcd30f7bf37, 0xe21e06a91917145e, ALL),
    ("ca", "~ \u{1}V ~ . a", "t'idl^@\nt'idl^@ @", "tˈidʎə\ntˈidʎə ə", 49480, 0x4dcad9547fcfdc7e, 0x978b15668f1bcff9, ALL),
    ("ca", "Hi. \u{1}V ok", "'i\n\n'Ok", "ˈi\n\nˈɔk", 43580, 0xf1d532c5436cbf15, 0x1c3d93a1479c3086, ALL),
    ("ca", "Hi. . ?~", "'i\nt'idl^@", "ˈi\ntˈidʎə", 50488, 0x872bad5cae405547, 0x71ba323e81457876, X | IPA | WAV),
    ("ca", "Mr. . ?~", ",em@'eR@\nt'idl^@", "ˌeməˈerə\ntˈidʎə", 62350, 0x3522ac1133e4fbd5, 0xd6306e82bab29d10, X | IPA | WAV),
    ("ca", "Hi. ?~", "'i\nt'idl^@", "ˈi\ntˈidʎə", 50488, 0x872bad5cae405547, 0xf72189b263a8af9c, X | IPA | WAV),
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
fn voice_clause_and_stray_marks_match_c() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = ROWS.iter().flat_map(differences).collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
