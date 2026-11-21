// tests/time_colon_reading.rs
//
// C keeps a `.` that does not end the clause (a lowercase word follows, or an SSML tag touches
// it) in the clause text, so the digit string before it is the word `30.`, not `30`.  The
// en rule `D_) : (_DD_` that drops the colon of a time needs the digits to end at a word
// break, so `10:30. b` says "colon" where `10:30 b` and `10:30.` do not.  Expected values are
// what C espeak-ng master (f0fb11a8) produces: `-q -x`, `-q --ipa`, the PCM of `-w` (byte
// count and FNV-1a 64 hash) and, for plain text, the word/sentence/end events (FNV-1a 64 hash
// of the transcript lines `ev <pos> <len> <ms> <sample> <type> num=<n>` and `samples=<n>`).

use std::path::Path;
use std::process::Command;

use espeak_ng::{EspeakNg, EventKind};

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn cli(voice: &str, ssml: bool) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_espeak_cli"));
    cmd.args(["-v", voice]);
    if ssml {
        cmd.arg("-m");
    }
    cmd
}

fn text(voice: &str, ssml: bool, flag: &str, input: &str) -> String {
    let out = cli(voice, ssml).args(["-q", flag, "--", input]).output().expect("run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn pcm(voice: &str, ssml: bool, input: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("time_colon_reading_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(format!("{voice}{input}").as_bytes())));
    let status = cli(voice, ssml).arg("-w").arg(&path).args(["--", input]).status().expect("run");
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

/// (voice, input, SSML, `-x`, `--ipa`, PCM bytes, PCM hash, transcript hash, check events)
type Row = (&'static str, &'static str, bool, &'static str, &'static str, usize, u64, u64, bool);

const ROWS: &[Row] = &[
    ("en", "10:30. b", false, "t'En k'oUl@n T'3:ti b'i:", "tˈɛn kˈəʊlən θˈɜːti bˈiː", 71802, 0x9c6cf2e843906ab2, 0xa6843ad7581f4d1a, true),
    ("en", "<speak>10:30. b</speak>", true, "t'En k'oUl@n T'3:ti b'i:", "tˈɛn kˈəʊlən θˈɜːti bˈiː", 72110, 0xdc5f2afb0721fc52, 0, false),
    ("en", "10:30.", false, "t'En T'3:ti", "tˈɛn θˈɜːti", 44238, 0x9c4f2b5335e42795, 0x790e9c9695abc506, true),
    ("en", "<speak>10:30.</speak>", true, "t'En k'oUl@n T'3:ti", "tˈɛn kˈəʊlən θˈɜːti", 61936, 0x1b47dff8c22ed914, 0, false),
    ("en", "10:30 b", false, "t'En T'3:ti b'i:", "tˈɛn θˈɜːti bˈiː", 54526, 0x5e9bcd90f8fe2cc1, 0x88df3d15469f05f9, true),
    ("en", "<speak>10:30 b</speak>", true, "t'En T'3:ti b'i:", "tˈɛn θˈɜːti bˈiː", 54834, 0x2fd1b083ef204e11, 0, false),
    ("en", "1:2.", false, "w'0n k'oUl@n t'u:", "wˈɒn kˈəʊlən tˈuː", 55066, 0x40c8983300d35678, 0x7d8974697477591d, true),
    ("en", "<speak>1:2.</speak>", true, "w'0n k'oUl@n t'u:", "wˈɒn kˈəʊlən tˈuː", 55374, 0x0d8139f3819bb3f8, 0, false),
    ("en", "a 10:30. b", false, "a# t'En k'oUl@n T'3:ti b'i:", "ɐ tˈɛn kˈəʊlən θˈɜːti bˈiː", 75004, 0x0ec738c5ce09768c, 0xf3bab4f58179f6a7, true),
    ("en", "<speak>a 10:30. b</speak>", true, "a# t'En k'oUl@n T'3:ti b'i:", "ɐ tˈɛn kˈəʊlən θˈɜːti bˈiː", 75312, 0xffa2f1d3c353be4c, 0, false),
    ("en", "12:00:00. x", false, "tw'Elv z'i@roU z'i@roU k'oUl@n z'i@roU z'i@roU 'Eks", "twˈɛlv zˈiəɹəʊ zˈiəɹəʊ kˈəʊlən zˈiəɹəʊ zˈiəɹəʊ ˈɛks", 119578, 0x07ffc3f54853c3d5, 0x72ab1bf00a2c6e19, true),
    ("en", "<speak>12:00:00. x</speak>", true, "tw'Elv z'i@roU z'i@roU k'oUl@n z'i@roU z'i@roU 'Eks", "twˈɛlv zˈiəɹəʊ zˈiəɹəʊ kˈəʊlən zˈiəɹəʊ zˈiəɹəʊ ˈɛks", 119886, 0xf5191a085abfff65, 0, false),
    ("en", "3:45pm. y", false, "Tr'i: f'o@ti f'aIv p,i:;'Em w'aI", "θɹˈiː fˈɔːti fˈaɪv pˌiːˈɛm wˈaɪ", 81784, 0xd2893f3a5e9dd6ff, 0x95e1d70ae7fa1b87, true),
    ("en", "<speak>3:45pm. y</speak>", true, "Tr'i: f'o@ti f'aIv p,i:;'Em w'aI", "θɹˈiː fˈɔːti fˈaɪv pˌiːˈɛm wˈaɪ", 82092, 0x2a77f13a03b7a1af, 0, false),
    ("de", "10:30. b", false, "ts'e:n dr'aIsICst@ b'e:", "tsˈeːn dɾˈaɪsɪçstə bˈeː", 69736, 0x410d35307635378e, 0x62172c8d6a3cc2b7, true),
    ("de", "<speak>10:30. b</speak>", true, "ts'e:n dr'aIsICst@ b'e:", "tsˈeːn dɾˈaɪsɪçstə bˈeː", 70044, 0x3bd43a43d46999ee, 0, false),
    ("de", "10:30.", false, "ts'e:n _|u:r dr'aIsIC", "tsˈeːn uːɾ dɾˈaɪsɪç", 55088, 0x1092c91dd58e1315, 0x8c99db2f99965711, true),
    ("de", "<speak>10:30.</speak>", true, "ts'e:n dr'aIsICst@", "tsˈeːn dɾˈaɪsɪçstə", 58334, 0x77074a273b719f72, 0, false),
    ("de", "10:30 b", false, "ts'e:n _|u:r dr'aIsIC b'e:", "tsˈeːn uːɾ dɾˈaɪsɪç bˈeː", 65718, 0xa1db756d4eccfe8d, 0xbfa9da19dc12f768, true),
    ("de", "<speak>10:30 b</speak>", true, "ts'e:n _|u:r dr'aIsIC b'e:", "tsˈeːn uːɾ dɾˈaɪsɪç bˈeː", 66026, 0x749766bd4be62b9d, 0, false),
    ("de", "1:2.", false, "_'aIns tsv'aI", "ˈaɪns tsvˈaɪ", 45808, 0xd3d9fd859952f399, 0xc07090c338ded820, true),
    ("de", "<speak>1:2.</speak>", true, "_'aIns tsv'aIt@", "ˈaɪns tsvˈaɪtə", 48322, 0x5dfb5cf61cc859e0, 0, false),
    ("de", "a 10:30. b", false, "_!'A: ts'e:n dr'aIsICst@ b'e:", "ˈɑː tsˈeːn dɾˈaɪsɪçstə bˈeː", 76938, 0x7d0ae3c6d0bcec94, 0x2caea5a7a262e46c, true),
    ("de", "<speak>a 10:30. b</speak>", true, "_!'A: ts'e:n dr'aIsICst@ b'e:", "ˈɑː tsˈeːn dɾˈaɪsɪçstə bˈeː", 77246, 0x177efff6240d3ed4, 0, false),
    ("de", "12:00:00. x", false, "tsv'Wlf _|u:r n'Ul n'Ul n'Ul n'Ul _!'Iks", "tsvˈœlf uːɾ nˈʊl nˈʊl nˈʊl nˈʊl ˈɪks", 96102, 0xebc4048e02ba756a, 0x96c82bac79f689e2, true),
    ("de", "<speak>12:00:00. x</speak>", true, "tsv'Wlf _|u:r n'Ul n'Ul n'Ul n'Ul _!'Iks", "tsvˈœlf uːɾ nˈʊl nˈʊl nˈʊl nˈʊl ˈɪks", 96410, 0xfd0b912eea49f88a, 0, false),
    ("de", "3:45pm. y", false, "dr'aI _|u:r f'ynf _|Untf'IrtsIC p,e:'Em _!'ypsIl,On", "dɾˈaɪ uːɾ fˈʏnf ʊntfˈɪɾtsɪç pˌeːˈɛm ˈʏpsɪlˌɔn", 108308, 0xdfeef2c99558f3e1, 0x3e7dcb036a5329e8, true),
    ("de", "<speak>3:45pm. y</speak>", true, "dr'aI _|u:r f'ynf _|Untf'IrtsIC p,e:'Em _!'ypsIl,On", "dɾˈaɪ uːɾ fˈʏnf ʊntfˈɪɾtsɪç pˌeːˈɛm ˈʏpsɪlˌɔn", 108616, 0x20617a114de8ef31, 0, false),
    ("fr", "10:30. b", false, "d'i tr'A~t b'e", "dˈi tʁˈɑ̃t bˈe", 39996, 0xfe86256daac46eb8, 0x1082827d7d587a95, true),
    ("fr", "<speak>10:30. b</speak>", true, "d'i tr'A~t b'e", "dˈi tʁˈɑ̃t bˈe", 40304, 0x9b7e92fe0875a038, 0, false),
    ("fr", "10:30.", false, "d'iz 'Wr tr'A~t", "dˈiz ˈœʁ tʁˈɑ̃t", 43932, 0x39d9f0ec63b2fece, 0xa8e04e7f0f1beaa3, true),
    ("fr", "<speak>10:30.</speak>", true, "d'i tr'A~t", "dˈi tʁˈɑ̃t", 34308, 0xac5b8b2f8b9d1676, 0, false),
    ("fr", "10:30 b", false, "d'iz 'Wr tr'A~t b'e", "dˈiz ˈœʁ tʁˈɑ̃t bˈe", 49766, 0x9414330c1dc99740, 0x49b602af8e0c0557, true),
    ("fr", "<speak>10:30 b</speak>", true, "d'iz 'Wr tr'A~t b'e", "dˈiz ˈœʁ tʁˈɑ̃t bˈe", 50074, 0x304551e48c751b40, 0, false),
    ("fr", "1:2.", false, "'W~ d'Y", "ˈœ̃ dˈø", 27346, 0xb0ec22ef760b166b, 0x1d6c849c62b56f57, true),
    ("fr", "<speak>1:2.</speak>", true, "'W~ d'Y", "ˈœ̃ dˈø", 27654, 0x50a4b413085e4edb, 0, false),
    ("fr", "a 10:30. b", false, "a d'i tr'A~t b'e", "a dˈi tʁˈɑ̃t bˈe", 45018, 0xd56aed1a458704e5, 0x2bfbedb09e553222, true),
    ("fr", "<speak>a 10:30. b</speak>", true, "a d'i tr'A~t b'e", "a dˈi tʁˈɑ̃t bˈe", 45326, 0xcf92b6f31d62e575, 0, false),
    ("fr", "12:00:00. x", false, "d'uz 'Wr zero zer'o zero zer'o _!'iks", "dˈuz ˈœʁ zeʁo zeʁˈo zeʁo zeʁˈo ˈiks", 92958, 0x129154b87d4116f9, 0x67a270332beadcf1, true),
    ("fr", "<speak>12:00:00. x</speak>", true, "d'uz 'Wr zero zer'o zero zer'o _!'iks", "dˈuz ˈœʁ zeʁo zeʁˈo zeʁo zeʁˈo ˈiks", 93266, 0x09e260d1795481c9, 0, false),
    ("fr", "3:45pm. y", false, "trw'az2 'Wr karA~ts'E~k p,e'Em _|igr'Ek", "tʁwˈaz ˈœʁ kaʁɑ̃tsˈɛ̃k pˌeˈɛm iɡʁˈɛk", 91250, 0xe04b12d4a77393d2, 0xcec8683a1d2622ac, true),
    ("fr", "<speak>3:45pm. y</speak>", true, "trw'az2 'Wr karA~ts'E~k p,e'Em _|igr'Ek", "tʁwˈaz ˈœʁ kaʁɑ̃tsˈɛ̃k pˌeˈɛm iɡʁˈɛk", 91558, 0xa6421e53eebb9f72, 0, false),
    ("es", "10:30. b", false, "dj'ET t**'eInta B'e", "djˈɛθ tɾˈeɪnta βˈe", 49554, 0x0b74871b4d63e470, 0xaee449fef6de4d6f, true),
    ("es", "10:30.", false, "dj'ET t**'eInta", "djˈɛθ tɾˈeɪnta", 43054, 0x6e3b208d54661d2a, 0xe1367ba5c1bf5d7d, true),
    ("es", "10:30 b", false, "dj'ET t**'eInta B'e", "djˈɛθ tɾˈeɪnta βˈe", 49554, 0x0b74871b4d63e470, 0x88c2e68a8089dc98, true),
    ("es", "1:2.", false, "'uno D'os", "ˈuno ðˈos", 38170, 0x0789407cc19829f2, 0x2b6d06f363ced543, true),
    ("es", "a 10:30. b", false, "a Dj'ET t**'eInta B'e", "a ðjˈɛθ tɾˈeɪnta βˈe", 55456, 0x9ef730635a896f91, 0x0d9ef760e06bbcde, true),
    ("es", "12:00:00. x", false, "d'oTe T'e**o T'e**o T'e**o T'e**o 'Ekis", "dˈoθe θˈeɾo θˈeɾo θˈeɾo θˈeɾo ˈɛkis", 90572, 0x549a1881d32f1c14, 0x525e028d87f156eb, true),
    ("es", "3:45pm. y", false, "t**'es kwa**,EntaiT'inko p,e'Eme i", "tɾˈes kwaɾˌɛntaiθˈinko pˌeˈɛme i", 83466, 0x15c86a2a93335345, 0x2da06c894ae0e73e, true),
    ("ru", "10:30. b", false, "d;'es;It; t@-r;'i_:tsVt; (en)b'i:(ru)", "dʲˈesʲɪtʲ trʲˈitsʌtʲ (en)bˈiː(ru)", 63710, 0x78825f99af825ffc, 0x5f73380f1f26c160, true),
    ("ru", "<speak>10:30. b</speak>", true, "d;'es;It; t@-r;'i_:tsVt; (en)b'i:(ru)", "dʲˈesʲɪtʲ trʲˈitsʌtʲ (en)bˈiː(ru)", 64018, 0xd7ec0b325feecabc, 0, false),
    ("ru", "10:30.", false, "d;'es;It; t@-r;'i_:tsVt;", "dʲˈesʲɪtʲ trʲˈitsʌtʲ", 50644, 0xf5b3e9a115ca9699, 0x33b5fcb7b325c7a7, true),
    ("ru", "<speak>10:30.</speak>", true, "d;'es;It; t@-r;'i_:tsVt;", "dʲˈesʲɪtʲ trʲˈitsʌtʲ", 50952, 0x8ee7d92fb7b00369, 0, false),
    ("ru", "10:30 b", false, "d;'es;It; t@-r;'i_:tsVt; (en)b'i:(ru)", "dʲˈesʲɪtʲ trʲˈitsʌtʲ (en)bˈiː(ru)", 63710, 0x78825f99af825ffc, 0xa1b4e0ddc2a63077, true),
    ("ru", "<speak>10:30 b</speak>", true, "d;'es;It; t@-r;'i_:tsVt; (en)b'i:(ru)", "dʲˈesʲɪtʲ trʲˈitsʌtʲ (en)bˈiː(ru)", 64018, 0xd7ec0b325feecabc, 0, false),
    ("ru", "1:2.", false, "V#d;'in_ dv'A", "ʌdʲˈin dvˈɑ", 35914, 0x3d0351c1fb468c4c, 0xc43d1ac2b8248b4b, true),
    ("ru", "<speak>1:2.</speak>", true, "V#d;'in_ dv'A", "ʌdʲˈin dvˈɑ", 36222, 0xb2d465c6b23c600c, 0, false),
    ("ru", "a 10:30. b", false, "(en)'eI(ru) d;'es;It; t@-r;'i_:tsVt; (en)b'i:(ru)", "(en)ˈeɪ(ru) dʲˈesʲɪtʲ trʲˈitsʌtʲ (en)bˈiː(ru)", 73212, 0xc9527371c398dba7, 0xce4f24b6b45c24aa, true),
    ("ru", "<speak>a 10:30. b</speak>", true, "(en)'eI(ru) d;'es;It; t@-r;'i_:tsVt; (en)b'i:(ru)", "(en)ˈeɪ(ru) dʲˈesʲɪtʲ trʲˈitsʌtʲ (en)bˈiː(ru)", 73520, 0xf6153ff227da62d7, 0, false),
    ("ru", "12:00:00. x", false, "dv;In'A_:tsVt; n'ol n'ol n'ol n'ol (en)'Eks(ru)", "dvʲɪnˈɑtsʌtʲ nˈoɭ nˈoɭ nˈoɭ nˈoɭ (en)ˈɛks(ru)", 80750, 0xbee20bdb750d5fd8, 0xc31cbc74508f6207, true),
    ("ru", "<speak>12:00:00. x</speak>", true, "dv;In'A_:tsVt; n'ol n'ol n'ol n'ol (en)'Eks(ru)", "dvʲɪnˈɑtsʌtʲ nˈoɭ nˈoɭ nˈoɭ nˈoɭ (en)ˈɛks(ru)", 81058, 0x6d589423de2b8b58, 0, false),
    ("ru", "3:45pm. y", false, "tR;'i s'o@-rV#kp;'At; (en)p,i:;'Em w'aI(ru)", "trʲˈi sˈorʌkpʲˈɑtʲ (en)pˌiːˈɛm wˈaɪ(ru)", 79146, 0x025199e22cc88995, 0x89491fe4024bee68, true),
    ("ru", "<speak>3:45pm. y</speak>", true, "tR;'i s'o@-rV#kp;'At; (en)p,i:;'Em w'aI(ru)", "trʲˈi sˈorʌkpʲˈɑtʲ (en)pˌiːˈɛm wˈaɪ(ru)", 79454, 0x5b3b7f2a7ab75125, 0, false),
    ("ku", "10:30. b", false, "d'Eh s'i b'e", "dˈɛh sˈi bˈe", 43816, 0x14e696d0329b114e, 0x6756b6224bb8c6cf, true),
    ("ku", "<speak>10:30. b</speak>", true, "d'Eh s'i b'e", "dˈɛh sˈi bˈe", 44124, 0x695a94f3548c3fae, 0, false),
    ("ku", "10:30.", false, "d'Eh s'i", "dˈɛh sˈi", 33998, 0x26304a7d52e7d125, 0x9d230f5e96dd7cae, true),
    ("ku", "<speak>10:30.</speak>", true, "d'Eh s'i", "dˈɛh sˈi", 34306, 0x694878749c98c5b5, 0, false),
    ("ku", "10:30 b", false, "d'Eh s'i b'e", "dˈɛh sˈi bˈe", 43816, 0x14e696d0329b114e, 0x8b3f7660276bf9d6, true),
    ("ku", "<speak>10:30 b</speak>", true, "d'Eh s'i b'e", "dˈɛh sˈi bˈe", 44124, 0x695a94f3548c3fae, 0, false),
    ("ku", "1:2.", false, "j'Ek d'U", "jˈɛk dˈʊ", 31936, 0x16186a1bde8e6bdd, 0xae8cc64daf2dcb70, true),
    ("ku", "<speak>1:2.</speak>", true, "j'Ek d'U", "jˈɛk dˈʊ", 32244, 0x3ca6384517fa51ed, 0, false),
    ("ku", "a 10:30. b", false, "a d'Eh s'i b'e", "a dˈɛh sˈi bˈe", 49334, 0xb4ceab595c1fd6e1, 0xe7cdf2b8fa331206, true),
    ("ku", "<speak>a 10:30. b</speak>", true, "a d'Eh s'i b'e", "a dˈɛh sˈi bˈe", 49642, 0x000252604e65c231, 0, false),
    ("ku", "12:00:00. x", false, "danzd'E# s'Ifr s'Ifr s'Ifr s'Ifr x'e", "danzdˈɛ sˈɪfr sˈɪfr sˈɪfr sˈɪfr xˈe", 96922, 0x5d1b47f8062b5049, 0x37a7610a5c30a83a, true),
    ("ku", "<speak>12:00:00. x</speak>", true, "danzd'E# s'Ifr s'Ifr s'Ifr s'Ifr x'e", "danzdˈɛ sˈɪfr sˈɪfr sˈɪfr sˈɪfr xˈe", 97230, 0xa1b05e36c3cd3419, 0, false),
    ("ku", "3:45pm. y", false, "s'e tS'Il_up'endZ p#m'e j'e", "sˈe tʃˈɪlupˈendʒ pʰmˈe jˈe", 78444, 0xeedec1ca664af19c, 0x68b7c06f07df8822, true),
    ("ku", "<speak>3:45pm. y</speak>", true, "s'e tS'Il_up'endZ p#m'e j'e", "sˈe tʃˈɪlupˈendʒ pʰmˈe jˈe", 78752, 0x45b53931ea58fe5c, 0, false),
    ("it", "10:30. b", false, "dj'EtSI t@-*'e:nta b'i", "djˈɛtʃɪ trˈeːnta bˈi", 56468, 0xe716a46ccb5499ec, 0x5e3dfc27900e77d4, true),
    ("it", "10:30.", false, "dj'EtSI _e_ t@-*'e:nta", "djˈɛtʃɪ e trˈeːnta", 53596, 0x16cca52fd3ba2e8b, 0xff50a7d34d35d2e9, true),
    ("it", "10:30 b", false, "dj'EtSI _e_ t@-*'e:nta b'i", "djˈɛtʃɪ e trˈeːnta bˈi", 61668, 0x505f4f15c41c7895, 0xae1ace5197ff0877, true),
    ("it", "1:2.", false, "'uno d'ue", "ˈuno dˈue", 34456, 0x1a2b46154cbbe1f2, 0x5c900ab320bd411d, true),
    ("it", "a 10:30. b", false, "a dj'EtSI t@-*'e:nta b'i", "a djˈɛtʃɪ trˈeːnta bˈi", 60970, 0x229b56fde4eb6847, 0x0549395d58707de5, true),
    ("it", "12:00:00. x", false, "d'oditSI _e_ dz'E*o dz'E*o dz'E*o dz'E*o_| 'iks", "dˈoditʃɪ e dzˈɛɾo dzˈɛɾo dzˈɛɾo dzˈɛɾo ˈiks", 98288, 0x13e42ee3e912fadc, 0x54e38b83e5e7bd78, true),
    ("it", "3:45pm. y", false, "t@-*'e _e_ kUa*'a:ntatS'inkwe p,i;'Emme_| 'ipsilon", "trˈe e kʊaɾˈaːntatʃˈinkwe pˌiˈɛmme ˈipsilon", 106938, 0xbed5b5a265c840e9, 0x614ce540e4c6a5fd, true),
    ("pt", "10:30. b", false, "d'Es# tR'iNt& b'e", "dˈɛʃ tɹˈiŋtɐ bˈe", 50540, 0xea34acf806dc4698, 0xae8ad4b51f4b62c0, true),
    ("pt", "10:30.", false, "d'Es# tR'iNt&", "dˈɛʃ tɹˈiŋtɐ", 41238, 0x29fb9bba6fc2978b, 0xd9ad59a799552710, true),
    ("pt", "10:30 b", false, "d'Es# tR'iNt& b'e", "dˈɛʃ tɹˈiŋtɐ bˈe", 50540, 0xea34acf806dc4698, 0x259ca159531daf7b, true),
    ("pt", "1:2.", false, "'um d'oIs#", "ˈum dˈoɪʃ", 38934, 0xf36250d04e205654, 0x62af7fcc0bceaf62, true),
    ("pt", "a 10:30. b", false, "& d'Es# tR'iNt& b'e", "ɐ dˈɛʃ tɹˈiŋtɐ bˈe", 56006, 0xc997403d56afa1df, 0x87ed5971a9b674a5, true),
    ("pt", "12:00:00. x", false, "d'ozy z'E*u z'E*u z'E*u z'E*u S'is#", "dˈozɨ zˈɛɾu zˈɛɾu zˈɛɾu zˈɛɾu ʃˈiʃ", 94032, 0xbd78bc5478678eb7, 0x9b72d267a2593ee4, true),
    ("pt", "3:45pm. y", false, "tR'es# kw&*'eNt&is'iNku p,e'Em 'ips,ilON", "tɹˈeʃ kwɐɾˈeŋtɐisˈiŋku pˌeˈɛm ˈipsˌilɔŋ", 105576, 0x44a2e902c97a49d0, 0x51339cc946a1e456, true),
    ("nl", "10:30. b", false, "t'in d'Ert@xst@ b'e:", "tˈin dˈɛrtəxstə bˈeː", 63468, 0x28d326c00fff18d7, 0xba1089ef6f889464, true),
    ("nl", "10:30.", false, "t'in d'Ert@x", "tˈin dˈɛrtəx", 44102, 0xfb8bbad420c6864a, 0x6586172395754815, true),
    ("nl", "10:30 b", false, "t'in d'Ert@x b'e:", "tˈin dˈɛrtəx bˈeː", 55398, 0xe5fa970d668914d6, 0x01415b887bab532d, true),
    ("nl", "1:2.", false, "_! 'e:n tv#'e:", "ˈeːn tʋˈeː", 42306, 0x5dc3a1dc391f4645, 0xab186a5413f8bddc, true),
    ("nl", "a 10:30. b", false, "_!'a: t'in d'Ert@xst@ b'e:", "ˈaː tˈin dˈɛrtəxstə bˈeː", 70522, 0x5bdb976cf4448bfd, 0xf721edd5b1f93547, true),
    ("nl", "12:00:00. x", false, "tv#'a:lf n'8l n,8l n'8l n,8l _!'Iks", "tʋˈaːlf nˈɵl nˌɵl nˈɵl nˌɵl ˈɪks", 84870, 0xca2a9a657cc3a972, 0x140446169c3ada7f, true),
    ("nl", "3:45pm. y", false, "dr'i v'EIfEnf,I:rt@x p,e:'Em _!'EI", "drˈi vˈɛɪfɛnfˌɪːrtəx pˌeːˈɛm ˈɛɪ", 85220, 0x7e4c933e30b09e05, 0x0004138005e22868, true),
    ("pl", "10:30. b", false, "dz;'ES;En^ts; tSydz;'ES;c;i b'E", "dʑˈɛɕɛɲtɕ tʃɨdʑˈɛɕcʲi bˈɛ", 74266, 0xd504462511b8627c, 0x9f3a00d803e988c3, true),
    ("pl", "10:30.", false, "dz;'ES;En^ts; tSydz;'ES;c;i", "dʑˈɛɕɛɲtɕ tʃɨdʑˈɛɕcʲi", 66636, 0x3c3f7edf28b2497a, 0x723971f586b0ae51, true),
    ("pl", "10:30 b", false, "dz;'ES;En^ts; tSydz;'ES;c;i b'E", "dʑˈɛɕɛɲtɕ tʃɨdʑˈɛɕcʲi bˈɛ", 74266, 0xd504462511b8627c, 0xd66f3b4caca9fb96, true),
    ("pl", "1:2.", false, "j'EdEn dv'a", "jˈɛdɛn dvˈa", 42482, 0xc351af2ff93a62f3, 0x93596200917b178b, true),
    ("pl", "a 10:30. b", false, "_!a_ dz;'ES;En^ts; tSydz;'ES;c;i b'E", "a dʑˈɛɕɛɲtɕ tʃɨdʑˈɛɕcʲi bˈɛ", 81156, 0x6230b234c3ea9df5, 0xffa4b6302ae47c84, true),
    ("pl", "12:00:00. x", false, "dvan'aS;ts;E z'ERO: z'ERO: z'ERO: z'ERO: 'iks", "dvanˈaɕtɕɛ zˈɛrɔː zˈɛrɔː zˈɛrɔː zˈɛrɔː ˈiks", 110216, 0x45462911ed7ac048, 0x6ea6041fc06c2760, true),
    ("pl", "3:45pm. y", false, "tS'y tStERdz;'ES;c;ip;'En^ts; p,E'Em 'igREk", "tʃˈɨ tʃtɛrdʑˈɛɕcʲipʲˈɛɲtɕ pˌɛˈɛm ˈiɡrɛk", 103124, 0x49a7791646e68765, 0x685a6f6b1b9ee175, true),
    ("uk", "10:30. b", false, "d'Esjat dB'i: kR'apkI tR'etsjat (en)b'i:(uk)", "dˈɛsjat dβˈiː krˈapkɪ trˈetsjat (en)bˈiː(uk)", 131682, 0x34b1b9e1743f4d4f, 0x33a710dc4e3f5abb, true),
    ("uk", "10:30.", false, "d'Esjat dB'i: kR'apkI tR'etsjat", "dˈɛsjat dβˈiː krˈapkɪ trˈetsjat", 116812, 0xd6109922b438dfbf, 0x329d8a47bcc83cb6, true),
    ("uk", "10:30 b", false, "d'Esjat dB'i: kR'apkI tR'etsjat (en)b'i:(uk)", "dˈɛsjat dβˈiː krˈapkɪ trˈetsjat (en)bˈiː(uk)", 131682, 0x34b1b9e1743f4d4f, 0xd46e868f2296c314, true),
    ("uk", "1:2.", false, "od'en dB'i: kR'apkI dB'a", "odˈen dβˈiː krˈapkɪ dβˈa", 96982, 0x37d703bafa33f4d7, 0xbeb7cb560b118640, true),
    ("uk", "a 10:30. b", false, "(en)'eI(uk) d'Esjat dB'i: kR'apkI tR'etsjat (en)b'i:(uk)", "(en)ˈeɪ(uk) dˈɛsjat dβˈiː krˈapkɪ trˈetsjat (en)bˈiː(uk)", 144376, 0xd0a6a7a353e39a3d, 0xa0dfcf5276b85a30, true),
    ("uk", "12:00:00. x", false, "dvan'atsjat dB'i: kR'apkI n'ul^ n'ul^ dB'i: kR'apkI n'ul^ n'ul^ (en)'Eks(uk)", "dvanˈatsjat dβˈiː krˈapkɪ nˈuʎ nˈuʎ dβˈiː krˈapkɪ nˈuʎ nˈuʎ (en)ˈɛks(uk)", 217884, 0x0a96a487c659a7d9, 0xfc9c4988bbb01bd2, true),
    ("uk", "3:45pm. y", false, "tR'I dB'i: kR'apkI s'oRokpj'at (en)p,i:;'Em w'aI(uk)", "trˈɪ dβˈiː krˈapkɪ sˈorokpjˈat (en)pˌiːˈɛm wˈaɪ(uk)", 152626, 0x1f2398f1010d7f58, 0xdae9ddade4b72d11, true),
    ("ja", "10:30. b", false, "d_z'u_! s'an d_zu_! _:(en)b'i:(ja)", "dzˈɯᵝ sˈän dzɯᵝ (en)bˈiː(ja)", 64384, 0x28ea74fc3b164f64, 0x28db7d4f14b1d558, true),
    ("ja", "<speak>10:30. b</speak>", true, "d_z'u_! s'an d_zu_! _:(en)b'i:(ja)", "dzˈɯᵝ sˈän dzɯᵝ (en)bˈiː(ja)", 64692, 0xef91e3a7901192a4, 0, false),
    ("ja", "10:30.", false, "d_z'u_! s'an d_zu_!", "dzˈɯᵝ sˈän dzɯᵝ", 51554, 0x90671e8274c382ea, 0xf9274ee62da18a63, true),
    ("ja", "<speak>10:30.</speak>", true, "d_z'u_! s'an d_zu_!", "dzˈɯᵝ sˈän dzɯᵝ", 51862, 0x23abc5a2337abe0a, 0, false),
    ("ja", "10:30 b", false, "d_z'u_! s'an d_zu_! _:(en)b'i:(ja)", "dzˈɯᵝ sˈän dzɯᵝ (en)bˈiː(ja)", 64384, 0x28ea74fc3b164f64, 0x691d9f7c7e42ff06, true),
    ("ja", "<speak>10:30 b</speak>", true, "d_z'u_! s'an d_zu_! _:(en)b'i:(ja)", "dzˈɯᵝ sˈän dzɯᵝ (en)bˈiː(ja)", 64692, 0xef91e3a7901192a4, 0, false),
    ("ja", "1:2.", false, "'it_si_! n'i:_!", "ˈitsi nˈiː", 45830, 0x3dbf91edfea48461, 0xc1041433ace04a1a, true),
    ("ja", "<speak>1:2.</speak>", true, "'it_si_! n'i:_!", "ˈitsi nˈiː", 46138, 0x488ecc2f34d427b1, 0, false),
    ("ja", "a 10:30. b", false, "_:(en)'eI(ja) d_z'u_! s'an d_zu_! _:(en)b'i:(ja)", "(en)ˈeɪ(ja) dzˈɯᵝ sˈän dzɯᵝ (en)bˈiː(ja)", 75340, 0x624a0de67923e860, 0x555d1a8beeecaf18, true),
    ("ja", "<speak>a 10:30. b</speak>", true, "_:(en)'eI(ja) d_z'u_! s'an d_zu_! _:(en)b'i:(ja)", "(en)ˈeɪ(ja) dzˈɯᵝ sˈän dzɯᵝ (en)bˈiː(ja)", 75648, 0xdd50cbb0a38a6660, 0, false),
    ("ja", "12:00:00. x", false, "d_z'u_!ni:_! r,ei_! r'ei_! r,ei_! r'ei_! _:(en)'Eks(ja)", "dzˈɯᵝniː ɹˌe̞i ɹˈe̞i ɹˌe̞i ɹˈe̞i (en)ˈɛks(ja)", 92606, 0x5044f7e71c16dac3, 0xfff809b5e6329d6c, true),
    ("ja", "<speak>12:00:00. x</speak>", true, "d_z'u_!ni:_! r,ei_! r'ei_! r,ei_! r'ei_! _:(en)'Eks(ja)", "dzˈɯᵝniː ɹˌe̞i ɹˈe̞i ɹˌe̞i ɹˈe̞i (en)ˈɛks(ja)", 92914, 0x8d0d8dde7aae26b3, 0, false),
    ("ja", "3:45pm. y", false, "s'an_! jon d_z'u_!go_! _:(en)p'i: _:'Em _:w'aI(ja)", "sˈän jo̞n dzˈɯᵝɡo̞ (en)pˈiː ˈɛm wˈaɪ(ja)", 95608, 0xd419a2384b6d8195, 0xa37e09aebb72a957, true),
    ("ja", "<speak>3:45pm. y</speak>", true, "s'an_! jon d_z'u_!go_! _:(en)p'i: _:'Em _:w'aI(ja)", "sˈän jo̞n dzˈɯᵝɡo̞ (en)pˈiː ˈɛm wˈaɪ(ja)", 95916, 0x2c56980ce2c1c925, 0, false),
    ("cmn", "10:30. b", false, "s.'i.35_|_| s'a55ns.i.35_|_| (en)bi:33(cmn)_|", "s.ˈi.ɜ sˈa5ns.i.ɜ (en)biːɜ(cmn)", 54750, 0x7263597ac0f96ee3, 0x4ee95a8e62fa8701, true),
    ("cmn", "10:30.", false, "s.'i.35_|_| s'a55ns.i.35_|_|", "s.ˈi.ɜ sˈa5ns.i.ɜ", 48378, 0x015ca9dff6aa632d, 0x691632a14e298e3c, true),
    ("cmn", "10:30 b", false, "s.'i.35_|_| s'a55ns.i.35_|_| (en)bi:33(cmn)_|", "s.ˈi.ɜ sˈa5ns.i.ɜ (en)biːɜ(cmn)", 54750, 0x7263597ac0f96ee3, 0xe63e42e2199053f2, true),
    ("cmn", "1:2.", false, "j'i55_| '@r51_|", "jˈi5 ˈər5", 30812, 0x92ef5ac7b8672e9d, 0xb11523cf517d12ca, true),
    ("cmn", "a 10:30. b", false, "(en)'eI55(cmn)_| s.'i.35_|_| s'a55ns.i.35_|_| (en)bi:33(cmn)_|", "(en)ˈeɪ5(cmn) s.ˈi.ɜ sˈa5ns.i.ɜ (en)biːɜ(cmn)", 61700, 0xba145a4eb4a108c2, 0xcfc94edbb60f2aaf, true),
    ("cmn", "12:00:00. x", false, "s.i.35_|'@r51_| li35N_| l'i35N_| li35N_| l'i35N_| (en)E33ks(cmn)_|", "s.i.ɜˈər5 liɜŋ lˈiɜŋ liɜŋ lˈiɜŋ (en)ɛɜks(cmn)", 92892, 0xbb22246102f42250, 0x25c447bd84ae08de, true),
    ("cmn", "3:45pm. y", false, "s'a55n_| s'i[51s.i.35_|w,u21_| (en)pi:44;E11m_| waI11(cmn)_|", "sˈa5n sˈi̪5s.i.ɜwˌu2 (en)piː4ɛ1m waɪ1(cmn)", 75888, 0x9eee14e92fff5da6, 0xa92f226d4f83a8cd, true),
];

const ALL: &[&str] = &["-x", "--ipa", "wav", "events"];
const AUDIO: &[&str] = &["wav", "events"];

/// Fields that differ from C for reasons the colon does not touch: de and nl read the ordinal
/// dot of `00.` as an ordinal where C does not (`0. b` differs too), uk spells a word-final
/// consonant differently, ja stresses spelled English letters differently, cmn's audio differs.
const KNOWN_OTHER_CAUSES: &[(&str, &str, &[&str])] = &[
    ("de", "12:00:00. x", ALL),
    ("de", "<speak>12:00:00. x</speak>", &["-x", "--ipa", "wav"]),
    ("nl", "12:00:00. x", ALL),
    ("uk", "10:30. b", ALL),
    ("uk", "10:30.", ALL),
    ("uk", "10:30 b", ALL),
    ("uk", "a 10:30. b", ALL),
    ("uk", "12:00:00. x", ALL),
    ("ja", "10:30 b", &["events"]),
    ("ja", "3:45pm. y", ALL),
    ("ja", "<speak>3:45pm. y</speak>", &["-x", "--ipa", "wav"]),
    ("cmn", "10:30. b", AUDIO),
    ("cmn", "10:30 b", AUDIO),
    ("cmn", "a 10:30. b", AUDIO),
    ("cmn", "12:00:00. x", AUDIO),
    ("cmn", "3:45pm. y", AUDIO),
];

fn known_other_cause(voice: &str, input: &str, field: &str) -> bool {
    KNOWN_OTHER_CAUSES.iter().any(|&(v, i, fields)| v == voice && i == input && fields.contains(&field))
}

fn differences(row: &Row) -> Vec<String> {
    let &(voice, input, ssml, x, ipa, bytes, hash, events, check_events) = row;
    let mut checks = vec![
        ("-x", text(voice, ssml, "-x", input) == x),
        ("--ipa", text(voice, ssml, "--ipa", input) == ipa),
    ];
    let audio = pcm(voice, ssml, input);
    checks.push(("wav", (audio.len(), fnv1a(&audio)) == (bytes, hash)));
    if check_events {
        checks.push(("events", transcript_hash(voice, input) == events));
    }
    checks
        .into_iter()
        .filter(|&(field, same)| !same && !known_other_cause(voice, input, field))
        .map(|(field, _)| format!("{voice} {field} {input:?}"))
        .collect()
}

#[test]
fn colon_in_a_time_followed_by_a_kept_dot_is_read_as_c_reads_it() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = ROWS.iter().flat_map(differences).collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
