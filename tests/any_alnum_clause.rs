// tests/any_alnum_clause.rs
//
// C's `ReadClause` keeps one `any_alnum` flag per clause: set by any `iswalnum` character read
// (`²` and `½` too, though they never become words), cleared when a clause ends.  Until it is
// set a `.` is blanked and the clause goes on, and any other mark does not end the clause.  A
// newline after the mark skips the check, so the clause ends there.  Expected values are what
// C espeak-ng master (f0fb11a8) produces: `-q -x`, `-q --ipa`, the PCM of `-w` (byte count and
// FNV-1a 64 hash) and the word/sentence/end events (FNV-1a 64 hash of the transcript lines
// `ev <pos> <len> <ms> <sample> <type> num=<n>` and `samples=<n>`).
//
// Each row's last field says which of the four this port reproduces.  The others differ from C
// for reasons that have nothing to do with the flag: a `\n\n` paragraph pause, the pause after
// a Mandarin clause, the word event of `?~`, a one-letter clause's final pause.

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
    let dir = std::env::temp_dir().join(format!("any_alnum_clause_{}", std::process::id()));
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
    ("en", "~ ~ ~ ~. ~ ~ ~ ~", "t'Ild@ t'Ild@ t'Ild@", "tˈɪldə tˈɪldə tˈɪldə", 62118, 0x33fbba39824cf1c3, 0x628b1a85137e096c, ALL),
    ("en", ". ~", "t'Ild@", "tˈɪldə", 31974, 0x11c83743bc5e57a9, 0x530c20e74089f7ee, ALL),
    ("en", "Hi. . ~", "h'aI\nt'Ild@", "hˈaɪ\ntˈɪldə", 59904, 0x86f5a73f61ddee64, 0xd2d30ea3188db112, ALL),
    ("en", "Hi.\n. ~", "h'aI\nt'Ild@", "hˈaɪ\ntˈɪldə", 59904, 0x86f5a73f61ddee64, 0xd2d30ea3188db112, ALL),
    ("en", "~. ?", "t'Ild@", "tˈɪldə", 31974, 0x11c83743bc5e57a9, 0x663c6020d452cb21, ALL),
    ("en", "~.\n~", "t'Ild@\nt'Ild@", "tˈɪldə\ntˈɪldə", 63950, 0x97aaad8d06067ca4, 0x1800bbc637e36141, ALL),
    ("en", "~ ~.\n~ ~", "t'Ild@ t'Ild@\nt'Ild@ t'Ild@", "tˈɪldə tˈɪldə\ntˈɪldə tˈɪldə", 94256, 0x1659a8dbb43d6361, 0xf4409c340f3d9cf2, ALL),
    ("en", "~.\n\n~", "t'Ild@\nt'Ild@", "tˈɪldə\ntˈɪldə", 73916, 0x9a3d88935012170c, 0xdaca74c935c85dba, X | IPA),
    ("en", "². ~", "t'u:\nt'Ild@", "tˈuː\ntˈɪldə", 59628, 0xb7d2c8ce96288911, 0xfe8a7b80d255f785, ALL),
    ("en", "Hi. . ?~", "h'aI\nt'Ild@", "hˈaɪ\ntˈɪldə", 59904, 0x86f5a73f61ddee64, 0xd8e817087a4ec5bc, X | IPA | WAV),
    ("en", "Mr. . ?~", "m'Ist3\nt'Ild@", "mˈɪstə\ntˈɪldə", 47598, 0xb0dca13a06d72223, 0x08dd1135a195d353, X | IPA | WAV),
    ("en", "~.", "t'Ild@", "tˈɪldə", 31974, 0x11c83743bc5e57a9, 0x4356cddbd9d8cb73, ALL),
    ("en", "~ ~ ~.", "t'Ild@ t'Ild@ t'Ild@", "tˈɪldə tˈɪldə tˈɪldə", 62118, 0x33fbba39824cf1c3, 0x25e1ea21a0fce983, ALL),
    ("en", "+.", "pl'Vs", "plˈʌs", 29660, 0xe887ec4b62da082a, 0xc003a35532cdf9f2, ALL),
    ("en", "=.", "_:'i:kw@Lz_:", "ˈiːkwəlz", 40336, 0x4e8a21e7bf4a34d1, 0xdc86eb89714dd939, ALL),
    ("en", "/ .  ...", "sl'aS", "slˈaʃ", 30018, 0x307de1a3622aa767, 0x69f482bd59e5cd3b, ALL),
    ("en", "~ ?", "t'Ild@", "tˈɪldə", 31974, 0x11c83743bc5e57a9, 0xcd680a4972229c62, ALL),
    ("en", "a.", "'eI", "ˈeɪ", 8742, 0xa6866d33100b7bfa, 0x9cf835ba1ce1fff1, X | IPA),
    ("en", "1.", "w'0n", "wˈɒn", 28146, 0x616126cbc238d361, 0x96ac8fd7b066b720, ALL),
    ("en", "etc.", "Ets'Etr@", "ɛtsˈɛtɹə", 37524, 0x3fa701a2774e8045, 0xdb8173a5f0bb3f30, ALL),
    ("en", "... ~", "t'Ild@", "tˈɪldə", 31974, 0x11c83743bc5e57a9, 0x50c26619550f7c48, ALL),
    ("en", "x .", "'Eks", "ˈɛks", 12716, 0xf91ee8109dad44f3, 0x3e9abd537de3f366, X | IPA),
    ("en", ". .", "", "", 308, 0xc10fa6af0a66b7b5, 0x82f96cafd2c67fb7, ALL),
    ("en", ".a", "d'0t 'eI", "dˈɒt ˈeɪ", 31922, 0xb1ab2c6e9b1e977d, 0x7eea0753258958a9, ALL),
    ("en", "½. ~", "a# h'A:f\nt'Ild@", "ɐ hˈɑːf\ntˈɪldə", 65644, 0x1cf1b757566c4b1d, 0x7ae084a3a5296d4e, ALL),
    ("en", "~ ~ ~. ~ ~.", "t'Ild@ t'Ild@ t'Ild@", "tˈɪldə tˈɪldə tˈɪldə", 62118, 0x33fbba39824cf1c3, 0x92f77022d1faf0cd, ALL),
    ("en", "~ . ~", "t'Ild@ t'Ild@", "tˈɪldə tˈɪldə", 47122, 0x1c2fe3618525a57b, 0x79a9386c1e784a5f, ALL),
    ("en", "Hi. ~. ~", "h'aI\nt'Ild@ t'Ild@", "hˈaɪ\ntˈɪldə tˈɪldə", 75056, 0x0a000cdf8663b07f, 0x18aa3c2ebfa2315b, ALL),
    ("en", "~.\n. ~", "t'Ild@\nt'Ild@", "tˈɪldə\ntˈɪldə", 63950, 0x97aaad8d06067ca4, 0x281d1c08375770cb, ALL),
    ("en", "~? ~", "t'Ild@ t'Ild@", "tˈɪldə tˈɪldə", 47122, 0x1c2fe3618525a57b, 0xcf791d209c0b5658, ALL),
    ("en", "~, ~", "t'Ild@ t'Ild@", "tˈɪldə tˈɪldə", 47122, 0x1c2fe3618525a57b, 0xcf791d209c0b5658, ALL),
    ("en", "?.", "", "", 13274, 0xc3bcf1d6f6c00dcd, 0x77c6abcbf2bef5cc, ALL),
    ("en", ", .", "", "", 13274, 0xc3bcf1d6f6c00dcd, 0x2e48286e20181f15, ALL),
    ("de", ". ~", "t'Ild@", "tˈɪldə", 31462, 0xc6f1baa6638999de, 0x15a84ccd8b955a35, ALL),
    ("de", "Hi. . ~", "h'aI\nt'Ild@", "hˈaɪ\ntˈɪldə", 58398, 0x3ef3dd6ebea3f554, 0xea0b2060e87b0d50, ALL),
    ("de", "~. ?", "t'Ild@", "tˈɪldə", 31462, 0xc6f1baa6638999de, 0x3311d62cd97e208e, ALL),
    ("de", "~.\n~", "t'Ild@\nt'Ild@", "tˈɪldə\ntˈɪldə", 62936, 0x9fc0bb7866a488ce, 0xc2a2dc9c8e1d51a1, ALL),
    ("de", "~.\n\n~", "t'Ild@\nt'Ild@", "tˈɪldə\ntˈɪldə", 72902, 0x2ae4c1be70d4ae5e, 0x2ee1a1552227e30f, X | IPA),
    ("de", "². ~", "tsv'aI\nt'Ild@", "tsvˈaɪ\ntˈɪldə", 65746, 0x14456ab78b28b8dd, 0xa0c226194e22a1cc, ALL),
    ("de", "Hi. . ?~", "h'aI\nt'Ild@", "hˈaɪ\ntˈɪldə", 58398, 0x3ef3dd6ebea3f554, 0x0bb72d3e363dbb9e, X | IPA | WAV),
    ("de", "Mr. . ?~", "_!,Em'Er\nt'Ild@", "ˌɛmˈɛɾ\ntˈɪldə", 59386, 0xa43d869d36100872, 0x7239dde86503f2ef, X | IPA | WAV),
    ("de", "~.", "t'Ild@", "tˈɪldə", 31462, 0xc6f1baa6638999de, 0x955ac8ee494293a8, ALL),
    ("de", "+.", "pl'Us", "plˈʊs", 29152, 0x5d81cbb790fa8e8e, 0x97593b15d2f6ad06, ALL),
    ("de", "/ .  ...", "", "", 13274, 0xc3bcf1d6f6c00dcd, 0x4d7e3a4436cdd246, ALL),
    ("fr", ". ~", "tild'e", "tildˈe", 30814, 0xec0b3ce85eff3e5f, 0xb1e79ddd7804c983, ALL),
    ("fr", "Hi. . ~", "_|'i\ntild'e", "ˈi\ntildˈe", 50038, 0x60e93ac094623c6f, 0x73a0f2a3830c18f4, ALL),
    ("fr", "~. ?", "tild'e", "tildˈe", 30814, 0xec0b3ce85eff3e5f, 0x2a39497d90acb300, ALL),
    ("fr", "~.\n~", "tild'e\ntild'e", "tildˈe\ntildˈe", 61622, 0x2cab2ed29eae6278, 0x3ea0782110799949, ALL),
    ("fr", "~.\n\n~", "tild'e\ntild'e", "tildˈe\ntildˈe", 71588, 0x4de7bb25a94d02a0, 0x59691f03f45a77a8, X | IPA),
    ("fr", "². ~", "pyis'A~s d'Y\ntild'e", "pyisˈɑ̃s dˈø\ntildˈe", 71570, 0xf24cc9112eba2331, 0xe1084558d2bbe939, ALL),
    ("fr", "Hi. . ?~", "_|'i\ntild'e", "ˈi\ntildˈe", 50038, 0x60e93ac094623c6f, 0x36557c138fb1e1c0, X | IPA | WAV),
    ("fr", "Mr. . ?~", "m@sj'Y\ntild'e", "məsjˈø\ntildˈe", 62760, 0x4ed63a1348f516e5, 0xf51ffa30259a4836, X | IPA | WAV),
    ("fr", "~.", "tild'e", "tildˈe", 30814, 0xec0b3ce85eff3e5f, 0xf711acf946a4538e, ALL),
    ("fr", "+.", "pl'ys", "plˈys", 29278, 0x9d4c1b1ada03dc67, 0x819dd0380cdb5d78, ALL),
    ("fr", "/ .  ...", "", "", 13274, 0xc3bcf1d6f6c00dcd, 0x4d7e3a4436cdd246, ALL),
    ("es", "~ ~ ~ ~. ~ ~ ~ ~", "(en)t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@(es)", "(en)tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə(es)", 127876, 0xa0b409d180c4bdfb, 0x39030f04b9ac22c5, ALL),
    ("es", ". ~", "(en)t'Ild@(es)", "(en)tˈɪldə(es)", 30936, 0xb7f57d0fba40ce93, 0x76dd7dd01861767c, ALL),
    ("es", "Hi. . ~", "'i\n(en)t'Ild@(es)", "ˈi\n(en)tˈɪldə(es)", 51634, 0x806b7b62e48f9991, 0xf2fe8954c9aff8ef, ALL),
    ("es", "~. ?", "(en)t'Ild@(es)", "(en)tˈɪldə(es)", 30936, 0xb7f57d0fba40ce93, 0xbe4aaa371ecc704f, ALL),
    ("es", "~.\n~", "(en)t'Ild@(es)\n(en)t'Ild@(es)", "(en)tˈɪldə(es)\n(en)tˈɪldə(es)", 61866, 0xfb4e4ef158a791e8, 0x8b353d71a6369c52, ALL),
    ("es", "~.\n\n~", "(en)t'Ild@(es)\n(en)t'Ild@(es)", "(en)tˈɪldə(es)\n(en)tˈɪldə(es)", 71832, 0xe4880ae3e7bbced0, 0x98989dca87078148, X | IPA),
    ("es", "². ~", "d'os\n(en)t'Ild@(es)", "dˈos\n(en)tˈɪldə(es)", 56602, 0x54a7aa38a04dcd4d, 0x74f4ee89a0dec9ac, ALL),
    ("es", "Hi. . ?~", "'i\n(en)t'Ild@(es)", "ˈi\n(en)tˈɪldə(es)", 51634, 0x806b7b62e48f9991, 0x5bf369122be5b865, X | IPA | WAV),
    ("es", "Mr. . ?~", ",eme'ER2e\n(en)t'Ild@(es)", "ˌemeˈɛre\n(en)tˈɪldə(es)", 62858, 0xbd7ebee9f90ac63b, 0xa8141c2d6d25de03, X | IPA | WAV),
    ("es", "~.", "(en)t'Ild@(es)", "(en)tˈɪldə(es)", 30936, 0xb7f57d0fba40ce93, 0x705feaa1c36231cd, ALL),
    ("es", "+.", "m'as", "mˈas", 28198, 0x304784f3d355aab2, 0x4ed4489553329f1d, ALL),
    ("es", "/ .  ...", "b'aRR2a", "bˈara", 31644, 0x881ca024888b4cdf, 0xb05d20317887cc92, ALL),
    ("ru", ". ~", "t;'ilda#", "tʲˈiɭda", 24088, 0x6eee9d313d1566a1, 0x77c0539d7c564a31, ALL),
    ("ru", "Hi. . ~", "(en)h'aI(ru)\nt;'ilda#", "(en)hˈaɪ(ru)\ntʲˈiɭda", 52668, 0x89dd189d063eca3f, 0x2ecfd43d2eac611c, ALL),
    ("ru", "~. ?", "t;'ilda#", "tʲˈiɭda", 24088, 0x6eee9d313d1566a1, 0xd2b07772da710032, ALL),
    ("ru", "~.\n~", "t;'ilda#\nt;'ilda#", "tʲˈiɭda\ntʲˈiɭda", 48196, 0xb3c7731585398622, 0x914012b91a0460f7, ALL),
    ("ru", "~.\n\n~", "t;'ilda#\nt;'ilda#", "tʲˈiɭda\ntʲˈiɭda", 58162, 0x099f240fe71e50da, 0x6a948c83a47c6677, X | IPA),
    ("ru", "². ~", "dv'A\nt;'ilda#", "dvˈɑ\ntʲˈiɭda", 49538, 0xecd0be2a4d2cef36, 0xd10a6d973f824d66, ALL),
    ("ru", "Hi. . ?~", "(en)h'aI(ru)\nt;'ilda#", "(en)hˈaɪ(ru)\ntʲˈiɭda", 52668, 0x89dd189d063eca3f, 0x48ac729d482e5a12, X | IPA | WAV),
    ("ru", "Mr. . ?~", "(en)m'Ist3(ru)\nt;'ilda#", "(en)mˈɪstə(ru)\ntʲˈiɭda", 39920, 0x9b0b2d3827d82595, 0xb2ae07222a194890, X | IPA),
    ("ru", "~.", "t;'ilda#", "tʲˈiɭda", 24088, 0x6eee9d313d1566a1, 0x97e4ea6ce5c4cb90, ALL),
    ("ru", "+.", "pl;'u\"s", "pɭʲˈu\"s", 29310, 0xbb1df1fada658c5a, 0x7af3360088b2a157, ALL),
    ("ru", "/ .  ...", "kV#s'Aja tS;irt'A", "kʌsˈɑja tʃʲirtˈɑ", 44168, 0xb9196caa8c024d18, 0xe8b7c9bf5a6e2978, ALL),
    ("ja", "~ ~ ~ ~. ~ ~ ~ ~", "(en)t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@(ja)", "(en)tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə(ja)", 139452, 0x21141dc5a7a81626, 0x568c2923de7a6133, ALL),
    ("ja", ". ~", "(en)t'Ild@(ja)", "(en)tˈɪldə(ja)", 32384, 0xb4915f89ed1ee553, 0xfab683d79683bd78, ALL),
    ("ja", "Hi. . ~", "_:(en)'eItS _:'aI(ja)\n(en)t'Ild@(ja)", "(en)ˈeɪtʃ ˈaɪ(ja)\n(en)tˈɪldə(ja)", 74888, 0x25c0b07d202b9b21, 0x193ea5af44d96067, X | IPA | WAV),
    ("ja", "~. ?", "(en)t'Ild@(ja)", "(en)tˈɪldə(ja)", 32384, 0xb4915f89ed1ee553, 0x017dff58d2253807, ALL),
    ("ja", "~.\n~", "(en)t'Ild@(ja)\n(en)t'Ild@(ja)", "(en)tˈɪldə(ja)\n(en)tˈɪldə(ja)", 64776, 0x2721f191852a7416, 0x3cc3a8a7d9448abf, ALL),
    ("ja", "~.\n\n~", "(en)t'Ild@(ja)\n(en)t'Ild@(ja)", "(en)tˈɪldə(ja)\n(en)tˈɪldə(ja)", 74742, 0x2a131f8a59e2a9ae, 0x853ef2d929f16b3c, X | IPA),
    ("ja", "². ~", "n'i:_!\n(en)t'Ild@(ja)", "nˈiː\n(en)tˈɪldə(ja)", 62320, 0xd7af2b474b7ee22a, 0xbf2439832fc34f7e, ALL),
    ("ja", "Hi. . ?~", "_:(en)'eItS _:'aI(ja)\n(en)t'Ild@(ja)", "(en)ˈeɪtʃ ˈaɪ(ja)\n(en)tˈɪldə(ja)", 74888, 0x25c0b07d202b9b21, 0x09b59d52ea36f37d, X | IPA | WAV),
    ("ja", "~.", "(en)t'Ild@(ja)", "(en)tˈɪldə(ja)", 32384, 0xb4915f89ed1ee553, 0xc152c9186b77b531, ALL),
    ("ja", "+.", "(en)pl'Vs(ja)", "(en)plˈʌs(ja)", 30124, 0xba5c9b8fb1584f4e, 0xc013c64e877fb116, ALL),
    ("ja", "/ .  ...", "(en)sl'aS(ja)", "(en)slˈaʃ(ja)", 30370, 0xed65d042a40591fe, 0x49b74bd5e029c38f, ALL),
    ("cmn", "~ ~ ~ ~. ~ ~ ~ ~", "(en)t'I55ld@55_| t'I55ld@55_| t'I55ld@55_| t'I55ld@55_| t'I55ld@55_| t'I55ld@55_| t'I55ld@55_| t'I55ld@55(cmn)_|", "(en)tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5 tˈɪ5ldə5(cmn)", 134920, 0x08ed4f62e54d8a5e, 0x99c03ab27b987044, X | IPA),
    ("cmn", ". ~", "(en)t'I55ld@55(cmn)_|", "(en)tˈɪ5ldə5(cmn)", 30308, 0x59d346affd522f68, 0x87f6a17bd6d3213f, X | IPA),
    ("cmn", "Hi. . ~", "(en)h'aI55(cmn)_|\n(en)t'I55ld@55(cmn)_|", "(en)hˈaɪ5(cmn)\n(en)tˈɪ5ldə5(cmn)", 53728, 0xf518681ca6ff3125, 0x40a3eab339a73679, X | IPA),
    ("cmn", "~. ?", "(en)t'I55ld@55(cmn)_|", "(en)tˈɪ5ldə5(cmn)", 30308, 0x59d346affd522f68, 0x252e46fb2a5954fc, X | IPA),
    ("cmn", "~.\n~", "(en)t'I55ld@55(cmn)_|\n(en)t'I55ld@55(cmn)_|", "(en)tˈɪ5ldə5(cmn)\n(en)tˈɪ5ldə5(cmn)", 60614, 0x69b579c77b0eca8a, 0x4c8f30670d2ba9b7, X | IPA),
    ("cmn", "~.\n\n~", "(en)t'I55ld@55(cmn)_|\n(en)t'I55ld@55(cmn)_|", "(en)tˈɪ5ldə5(cmn)\n(en)tˈɪ5ldə5(cmn)", 70580, 0x863d92c0201be64a, 0x3e61b76bd8c09d36, X | IPA),
    ("cmn", "². ~", "'@r51_|\n(en)t'I55ld@55(cmn)_|", "ˈər5\n(en)tˈɪ5ldə5(cmn)", 52758, 0xfb8f5ffc9fc7d56b, 0xa4257b5b8be24875, X | IPA),
    ("cmn", "Hi. . ?~", "(en)h'aI55(cmn)_|\n(en)t'I55ld@55(cmn)_|", "(en)hˈaɪ5(cmn)\n(en)tˈɪ5ldə5(cmn)", 53728, 0xf518681ca6ff3125, 0xab54d3120704fea7, X | IPA),
    ("cmn", "Mr. . ?~", "(en)m'I55st355(cmn)_|\n(en)t'I55ld@55(cmn)_|", "(en)mˈɪ5stəə(cmn)\n(en)tˈɪ5ldə5(cmn)", 46898, 0x3060bb6392b972c5, 0xcdf5f8d60abaf1f6, X | IPA),
    ("cmn", "~.", "(en)t'I55ld@55(cmn)_|", "(en)tˈɪ5ldə5(cmn)", 30308, 0x59d346affd522f68, 0x547763670a0f1256, X | IPA),
    ("cmn", "+.", "(en)pl'V55s(cmn)_|", "(en)plˈʌ5s(cmn)", 27836, 0xf15b60db2d3f65c8, 0x0b6724f48efdfaf9, X | IPA),
    ("cmn", "/ .  ...", "(en)sl'a55S(cmn)_|", "(en)slˈa5ʃ(cmn)", 28106, 0x22e166ac6af49a38, 0xa9358157c6dfc0e5, X | IPA),
    ("ko", "~ ~ ~ ~. ~ ~ ~ ~", "(en)t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@ t'Ild@(ko)", "(en)tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə tˈɪldə(ko)", 139494, 0x7dbe659528c5a7e5, 0x2aba6ec545a36e81, ALL),
    ("ko", ". ~", "(en)t'Ild@(ko)", "(en)tˈɪldə(ko)", 32596, 0xcd095f32665d0aaa, 0xef0fb8ccd65c3051, ALL),
    ("ko", "Hi. . ~", "(en)h'aI(ko)\n(en)t'Ild@(ko)", "(en)hˈaɪ(ko)\n(en)tˈɪldə(ko)", 61166, 0x45c5cef9a486af72, 0xa28f43fef7ace6bf, ALL),
    ("ko", "~. ?", "(en)t'Ild@(ko)", "(en)tˈɪldə(ko)", 32596, 0xcd095f32665d0aaa, 0x5c7a8d3f5b272f06, ALL),
    ("ko", "~.\n~", "(en)t'Ild@(ko)\n(en)t'Ild@(ko)", "(en)tˈɪldə(ko)\n(en)tˈɪldə(ko)", 65194, 0x7e2a78e7bf228803, 0xc6a6e38b3e566ac8, ALL),
    ("ko", "~.\n\n~", "(en)t'Ild@(ko)\n(en)t'Ild@(ko)", "(en)tˈɪldə(ko)\n(en)tˈɪldə(ko)", 75160, 0xcaf68572315253d3, 0xa0776be6e84f1663, X | IPA),
    ("ko", "². ~", "'i_!\n(en)t'Ild@(ko)", "ˈi\n(en)tˈɪldə(ko)", 55874, 0xb09a40520e00f373, 0xac62a78f4444bf29, ALL),
    ("ko", "Hi. . ?~", "(en)h'aI(ko)\n(en)t'Ild@(ko)", "(en)hˈaɪ(ko)\n(en)tˈɪldə(ko)", 61166, 0x45c5cef9a486af72, 0xfe664ed3ef78e5d1, X | IPA | WAV),
    ("ko", "Mr. . ?~", "(en)m'Ist3(ko)\n(en)t'Ild@(ko)", "(en)mˈɪstə(ko)\n(en)tˈɪldə(ko)", 49302, 0xc34f1eb17ed5572b, 0x2e394b545da18870, X | IPA),
    ("ko", "~.", "(en)t'Ild@(ko)", "(en)tˈɪldə(ko)", 32596, 0xcd095f32665d0aaa, 0x3e057ae561dd5808, ALL),
    ("ko", "+.", "(en)pl'Vs(ko)", "(en)plˈʌs(ko)", 30314, 0xd2d59be3e738c514, 0x59af12171fee2e92, ALL),
    ("ko", "/ .  ...", "(en)sl'aS(ko)", "(en)slˈaʃ(ko)", 30722, 0xceb1fa53867d0ea5, 0x0d9e0490a860493e, ALL),
    ("ca", ". ~", "t'idl^@", "tˈidʎə", 30382, 0xfaadacf0aab39cb5, 0x43500aa0772541c0, ALL),
    ("ca", "Hi. . ~", "'i\nt'idl^@", "ˈi\ntˈidʎə", 50488, 0x872bad5cae405547, 0xb3583cb061c5d1b8, ALL),
    ("ca", "~. ?", "t'idl^@", "tˈidʎə", 30382, 0xfaadacf0aab39cb5, 0xa8062696a6130953, ALL),
    ("ca", "~.\n~", "t'idl^@\nt'idl^@", "tˈidʎə\ntˈidʎə", 60764, 0x94a91f5d832da0d6, 0xb55722bd65b09642, ALL),
    ("ca", "~.\n\n~", "t'idl^@\nt'idl^@", "tˈidʎə\ntˈidʎə", 70730, 0x1068ead53191fc8e, 0xed4dfc9100d44891, X | IPA),
    ("ca", "². ~", "d'os\nt'idl^@", "dˈos\ntˈidʎə", 56154, 0x196cf5ef61edab5c, 0xdc9c71ce940dc948, ALL),
    ("ca", "Hi. . ?~", "'i\nt'idl^@", "ˈi\ntˈidʎə", 50488, 0x872bad5cae405547, 0x71ba323e81457876, X | IPA | WAV),
    ("ca", "Mr. . ?~", ",em@'eR@\nt'idl^@", "ˌeməˈerə\ntˈidʎə", 62350, 0x3522ac1133e4fbd5, 0xd6306e82bab29d10, X | IPA | WAV),
    ("ca", "~.", "t'idl^@", "tˈidʎə", 30382, 0xfaadacf0aab39cb5, 0x96fe2066e6718b21, ALL),
    ("ca", "+.", "m'es", "mˈes", 27214, 0x576c5aa4db68e069, 0xae70e84737f22fb9, ALL),
    ("ca", "/ .  ...", "b'aR@", "bˈarə", 27424, 0x9fd30105b5736fa5, 0x52b2082729dd41a8, ALL),
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
fn clause_end_follows_any_alnum_as_c_does() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = ROWS.iter().flat_map(differences).collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
