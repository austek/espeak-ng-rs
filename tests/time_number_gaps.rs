// tests/time_number_gaps.rs
//
// Time and ordinal readings that depend on what C keeps in the clause buffer:
// a comma glued to the minute (`02:30,b`: the word is `30,` so the leading zero stays), a
// one-digit minute (`02:3 b`: C inserts a second space before a letter word, so the word
// after the colon is two characters wide and the zero goes), and the ordinal dot of de, nl,
// hu, da and no (a second dot or extra blanks before a lower-case word, an SSML tag before it,
// a newline after it, a leading zero before it).  Expected values are what C espeak-ng
// master (f0fb11a8) produces: `-q -x`, `-q --ipa`, the PCM of `-w` (byte count and FNV-1a
// 64 hash) and, for plain text, the word/sentence/end events (FNV-1a 64 hash of the
// transcript lines `ev <pos> <len> <ms> <sample> <type> num=<n>` and `samples=<n>`).  Events
// are left out for `no` (its word timing differs from C for `5. b` as well) and for the ja
// row where an English word follows (the word position differs for `10:30 b` too).

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
    let dir = std::env::temp_dir().join(format!("time_number_gaps_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{:x}.wav", fnv1a(format!("{voice}{input}{ssml}").as_bytes())));
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
    ("en", "02:30,b", false, "z'i@roU t'u: k'oUl@n T'3:ti b'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θˈɜːti bˈiː", 83814, 0xb5f456eaebfdd553, 0x5196285824f55ecd, true),
    ("en", "02:30,5", false, "z'i@roU t'u: k'oUl@n T'3:ti f'aIv", "zˈiəɹəʊ tˈuː kˈəʊlən θˈɜːti fˈaɪv", 88726, 0xec03f646561029d3, 0x7a9c694c90ddbf1b, true),
    ("en", "02:3. b", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0xf99635bcb743b8d9, true),
    ("en", "02:3 b", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0xfb56b404ee3eefee, true),
    ("en", "02:3,b", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0x0c2cec401be72043, true),
    ("en", "<speak>02:3. b</speak>", true, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64428, 0x4ce7054429278195, 0x0, false),
    ("en", "<speak>02:30,b</speak>", true, "z'i@roU t'u: k'oUl@n T'3:ti b'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θˈɜːti bˈiː", 84122, 0x7e623790825e3e43, 0x0, false),
    ("de", "02:30,b", false, "n'Ul tsv'aI dr'aIsIC b'e:", "nˈʊl tsvˈaɪ dɾˈaɪsɪç bˈeː", 71470, 0x6165635ccf9c99ac, 0x3dc33b9a9ecaa707, true),
    ("de", "02:30,5", false, "n'Ul tsv'aI dr'aIsIC _k,OmA: f'ynf", "nˈʊl tsvˈaɪ dɾˈaɪsɪç kˌɔmɑː fˈʏnf", 87350, 0x6ae40228538cb1b1, 0xee3dfd9a58f67790, true),
    ("de", "02:3. b", false, "tsv'aI dr'It@ b'e:", "tsvˈaɪ dɾˈɪtə bˈeː", 56970, 0x6eac1faeb2d6bd55, 0xc3250bca08f1a68f, true),
    ("de", "02:3 b", false, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53374, 0x81959187cf3f1920, 0xcfe7e55a4231ceca, true),
    ("de", "02:3,b", false, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53374, 0x81959187cf3f1920, 0xa085534ab7feb3ad, true),
    ("de", "<speak>02:3. b</speak>", true, "tsv'aI dr'It@ b'e:", "tsvˈaɪ dɾˈɪtə bˈeː", 57278, 0xa2e7a8156ca070e5, 0x0, false),
    ("de", "<speak>02:30,b</speak>", true, "n'Ul tsv'aI dr'aIsIC b'e:", "nˈʊl tsvˈaɪ dɾˈaɪsɪç bˈeː", 71778, 0xef498bfab53d7b6c, 0x0, false),
    ("fr", "02:30,b", false, "zero d'Y tr'A~t b'e", "zeʁo dˈø tʁˈɑ̃t bˈe", 53886, 0x59889b91a71ef86c, 0xc94cfd2de13ccf1f, true),
    ("fr", "02:30,5", false, "zero d'Y trA~t virgyl s'E~k", "zeʁo dˈø tʁɑ̃t viʁɡyl sˈɛ̃k", 74894, 0x64bf3f4edce47cbc, 0x99ef6a4f7ca42d5a, true),
    ("fr", "02:3. b", false, "d'Y trw'a b'e", "dˈø tʁwˈa bˈe", 38608, 0x83032102efadac1b, 0x3a8650720a7ad8f1, true),
    ("fr", "02:3 b", false, "d'Y trw'a b'e", "dˈø tʁwˈa bˈe", 38608, 0x83032102efadac1b, 0xc144146fac4fae9a, true),
    ("fr", "02:3,b", false, "d'Y trw'a b'e", "dˈø tʁwˈa bˈe", 38608, 0x83032102efadac1b, 0xc5334abe020efa81, true),
    ("fr", "<speak>02:3. b</speak>", true, "d'Y trw'a b'e", "dˈø tʁwˈa bˈe", 38916, 0xb9374f26ce1e1b8b, 0x0, false),
    ("es", "02:30,b", false, "T'e**o D'os t**'eInta B'e", "θˈeɾo ðˈos tɾˈeɪnta βˈe", 61152, 0x33059ba9bcc36ef2, 0xd5560b54a85e0534, true),
    ("es", "02:30,5", false, "T'e**o D'os t**'eInta k,oma T'inko", "θˈeɾo ðˈos tɾˈeɪnta kˌoma θˈinko", 82604, 0x9ab82705da69ba1b, 0x97ba236803dde809, true),
    ("es", "02:3. b", false, "d'os t**'es B'e", "dˈos tɾˈes βˈe", 42288, 0xe3b9d10a2e385e1e, 0x9d5554ebba6e6083, true),
    ("es", "02:3 b", false, "d'os t**'es B'e", "dˈos tɾˈes βˈe", 42288, 0xe3b9d10a2e385e1e, 0xa328e0ad776843b8, true),
    ("es", "02:3,b", false, "d'os t**'es B'e", "dˈos tɾˈes βˈe", 42288, 0xe3b9d10a2e385e1e, 0x4c3edcf88ef84f0f, true),
    ("ru", "02:30,b", false, "n'ol dv'A t@-r;'i_:tsVt; (en)b'i:(ru)", "nˈoɭ dvˈɑ trʲˈitsʌtʲ (en)bˈiː(ru)", 64916, 0x7c676cd80f7d090a, 0xa0edd1bc934f428d, true),
    ("ru", "02:30,5", false, "n'ol dv'A t@-r;'i_:tsVt; _:'i_: p;'At;_d;Is;'AtYx", "nˈoɭ dvˈɑ trʲˈitsʌtʲ ˈi pʲˈɑtʲdʲɪsʲˈɑtøx", 90534, 0x46e7f194c8b63594, 0xdfb67c4966c24cfd, true),
    ("ru", "02:3. b", false, "dv'A tR;'i (en)b'i:(ru)", "dvˈɑ trʲˈi (en)bˈiː(ru)", 44682, 0x7f0a428b1bf14b53, 0xa09bc0c07662362c, true),
    ("ru", "02:3 b", false, "dv'A tR;'i (en)b'i:(ru)", "dvˈɑ trʲˈi (en)bˈiː(ru)", 44682, 0x7f0a428b1bf14b53, 0x24956d41c6870fc5, true),
    ("ru", "02:3,b", false, "dv'A tR;'i (en)b'i:(ru)", "dvˈɑ trʲˈi (en)bˈiː(ru)", 44682, 0x7f0a428b1bf14b53, 0x8eb421dbdf8a955c, true),
    ("ku", "02:30,b", false, "s'Ifr d'U s'i b'e", "sˈɪfr dˈʊ sˈi bˈe", 53934, 0xe118105f7126a7ff, 0x2aff3440bc675020, true),
    ("ku", "02:3. b", false, "d'U s'e b'e", "dˈʊ sˈe bˈe", 40628, 0x076f626d1310e91d, 0xa7a87d45d0f2a3fc, true),
    ("ku", "02:3 b", false, "d'U s'e b'e", "dˈʊ sˈe bˈe", 40628, 0x076f626d1310e91d, 0x21e61fabfc88c953, true),
    ("it", "02:30,b", false, "dz'E*o d'ue t@-*'e:nta b'i", "dzˈɛɾo dˈue trˈeːnta bˈi", 63504, 0xec67c922f6971b91, 0xd0f1618954f05ed5, true),
    ("it", "02:3. b", false, "d'ue t@-*'e b'i", "dˈue trˈe bˈi", 43432, 0xfcc303ace7e9d79f, 0xcb5ac3f6b3d36f4f, true),
    ("it", "02:3 b", false, "d'ue t@-*'e b'i", "dˈue trˈe bˈi", 43432, 0xfcc303ace7e9d79f, 0x8f4071e055f26ba0, true),
    ("pt", "02:30,b", false, "z'E*u d'oIs# tR'iNt& b'e", "zˈɛɾu dˈoɪʃ tɹˈiŋtɐ bˈe", 67294, 0x6d55d59fb4464e1a, 0xb79e83ca8a799576, true),
    ("pt", "02:3. b", false, "d'oIs# tR'eZ b'e", "dˈoɪʃ tɹˈeʒ bˈe", 47594, 0x5070a9bfb81bce5b, 0xf761d4bcdb9b8bb6, true),
    ("pt", "02:3 b", false, "d'oIs# tR'eZ b'e", "dˈoɪʃ tɹˈeʒ bˈe", 47594, 0x5070a9bfb81bce5b, 0xf52320ca10aa08a3, true),
    ("nl", "02:30,b", false, "n'8l tv#,e: d'Ert@x b'e:", "nˈɵl tʋˌeː dˈɛrtəx bˈeː", 65894, 0x9bcf9b4cabe4e541, 0x236cbb6d12bcfed4, true),
    ("nl", "02:3. b", false, "tv#'e: d'Erd@ b'e:", "tʋˈeː dˈɛrdə bˈeː", 55074, 0x8191f0816a779299, 0x405b8da1f9749a46, true),
    ("nl", "02:3 b", false, "tv#'e: dr'i b'e:", "tʋˈeː drˈi bˈeː", 50252, 0xae958b65d921b78e, 0x47f926fd57c0f254, true),
    ("pl", "02:30,b", false, "z'ERO: dv'a tSydz;'ES;c;i b'E", "zˈɛrɔː dvˈa tʃɨdʑˈɛɕcʲi bˈɛ", 75768, 0x5dfff3e99d668a48, 0x95d78d59a1df6821, true),
    ("pl", "02:3. b", false, "dv'a tS'y b'E", "dvˈa tʃˈɨ bˈɛ", 44850, 0xedc36e92abaa96f0, 0x68526a66652cd9f0, true),
    ("pl", "02:3 b", false, "dv'a tS'y b'E", "dvˈa tʃˈɨ bˈɛ", 44850, 0xedc36e92abaa96f0, 0xebe80d39117fc345, true),
    ("uk", "02:30,b", false, "n'ul^ dB'a dB'i: kR'apkI tR'etsjat (en)b'i:(uk)", "nˈuʎ dβˈa dβˈiː krˈapkɪ trˈetsjat (en)bˈiː(uk)", 141250, 0xa3f7dca6cab1873b, 0x0b7a4b1bfb5c06c1, true),
    ("uk", "02:3. b", false, "dB'a dB'i: kR'apkI tR'I (en)b'i:(uk)", "dβˈa dβˈiː krˈapkɪ trˈɪ (en)bˈiː(uk)", 106530, 0xbe8f42b65d2c4ede, 0x4db1ff19d3e50aa5, true),
    ("uk", "02:3 b", false, "dB'a dB'i: kR'apkI tR'I (en)b'i:(uk)", "dβˈa dβˈiː krˈapkɪ trˈɪ (en)bˈiː(uk)", 106530, 0xbe8f42b65d2c4ede, 0xe50cc905165fe4c0, true),
    ("ja", "02:30,b", false, "re'i_! ni:_! s'an d_zu_! _:(en)b'i:(ja)", "ɹe̞ˈi niː sˈän dzɯᵝ (en)bˈiː(ja)", 72456, 0xc7d0cba3a212db6a, 0x1613d62e98a70ced, true),
    ("ja", "02:3. b", false, "n'i:_! s'an_! _:(en)b'i:(ja)", "nˈiː sˈän (en)bˈiː(ja)", 54698, 0xc45970fb54c2935d, 0x374114b293a6fd9e, true),
    ("ja", "02:3 b", false, "n'i:_! s'an_! _:(en)b'i:(ja)", "nˈiː sˈän (en)bˈiː(ja)", 54698, 0xc45970fb54c2935d, 0x0, false),
    ("de", "10:30.. b", false, "ts'e:n dr'aIsICst@ b'e:", "tsˈeːn dɾˈaɪsɪçstə bˈeː", 69736, 0x410d35307635378e, 0xc1fe2ade850a7e7d, true),
    ("de", "5.\nb", false, "f'ynf\nb'e:", "fˈʏnf\nbˈeː", 55236, 0x29e51a2fb4bdfeeb, 0x034bc3dc39bbbd3d, true),
    ("de", "5.. b", false, "f'ynft@ b'e:", "fˈʏnftə bˈeː", 45540, 0xc23479b2ce7d3b53, 0x29d6a079b36f944d, true),
    ("de", "5.  b", false, "f'ynft@ b'e:", "fˈʏnftə bˈeː", 45540, 0xc23479b2ce7d3b53, 0x29d6a079b36f944d, true),
    ("de", "05. b", false, "n'Ul f'ynf b'e:", "nˈʊl fˈʏnf bˈeː", 49910, 0x040a6560aceddbdd, 0xae170691c51e3c56, true),
    ("de", "00. b", false, "n'Ul n'Ul b'e:", "nˈʊl nˈʊl bˈeː", 47828, 0x8d83f7f5dd473581, 0x7260aa0b77f357f0, true),
    ("de", "0. b", false, "n'Ul b'e:", "nˈʊl bˈeː", 37010, 0xd1d68dd03a617436, 0x4e551946a47a7b7f, true),
    ("de", "5. B", false, "f'ynf\nb'e:", "fˈʏnf\nbˈeː", 55236, 0x29e51a2fb4bdfeeb, 0x034bc3dc39bbbd3d, true),
    ("nl", "10:30.. b", false, "t'in d'Ert@xst@ b'e:", "tˈin dˈɛrtəxstə bˈeː", 63468, 0x28d326c00fff18d7, 0xe3701a1c7b378752, true),
    ("nl", "5.\nb", false, "v'EIf\nb'e:", "vˈɛɪf\nbˈeː", 56490, 0x2afddec3e05098c7, 0x9edbf8580b5a5227, true),
    ("nl", "5.. b", false, "v'EIvd@ b'e:", "vˈɛɪvdə bˈeː", 45728, 0xff702ba2ab3c7c35, 0x979f179beeba79c5, true),
    ("nl", "5.  b", false, "v'EIvd@ b'e:", "vˈɛɪvdə bˈeː", 45728, 0xff702ba2ab3c7c35, 0x979f179beeba79c5, true),
    ("nl", "05. b", false, "n'8l v,EIf b'e:", "nˈɵl vˌɛɪf bˈeː", 47792, 0xad82772a824da7c5, 0xf2cf9864abd2de3f, true),
    ("nl", "00. b", false, "n'8l n,8l b'e:", "nˈɵl nˌɵl bˈeː", 45196, 0xf48a265c44279cf7, 0x7d37548438583a92, true),
    ("nl", "0. b", false, "n'8l b'e:", "nˈɵl bˈeː", 36260, 0xf581f62decedc8a7, 0x16d51500ced36536, true),
    ("hu", "10:30.. b", false, "t'i:z h'AR2_mintsAdik b'e:", "tˈiːz hˈɑrmintsɑdik bˈeː", 72982, 0xe65ccc17e3d93134, 0xbb6415f8a1504a8a, true),
    ("hu", "5.\nb", false, "_! 'Yt\nb'e:", "ˈøt\nbˈeː", 45444, 0x1a43758fa46a46b6, 0x002b848d1bb400cb, true),
    ("hu", "5.. b", false, "_! 'YtYdik b'e:", "ˈøtødik bˈeː", 43136, 0x71d529e48bf0fc6a, 0xd1ac424ab748cb1b, true),
    ("hu", "5.  b", false, "_! 'YtYdik b'e:", "ˈøtødik bˈeː", 43136, 0x71d529e48bf0fc6a, 0xd1ac424ab748cb1b, true),
    ("hu", "05. b", false, "n'ullA Yt b'e:", "nˈullɑ øt bˈeː", 43716, 0x031937e789c45f05, 0xe152c20b8102a1a1, true),
    ("hu", "00. b", false, "n'ullA nullA b'e:", "nˈullɑ nullɑ bˈeː", 49810, 0x9286931ff56c7345, 0x4b2b00471920461a, true),
    ("hu", "0. b", false, "n'ullA b'e:", "nˈullɑ bˈeː", 37638, 0x8139259a15df9292, 0x17609459ff7fc974, true),
    ("da", "10:30.. b", false, "t'?i tR'&#ft@- b'e", "tˈʔi tʁˈaftə bˈe", 49624, 0x690d23bd865914d7, 0xd4b72d1cae9bd035, true),
    ("da", "5.\nb", false, "f'?Em\nb'e", "fˈεm\nbˈe", 49328, 0xad595a157abbc251, 0xa39fcb0d17234c92, true),
    ("da", "5.. b", false, "f'Emt@- b'e", "fˈɛmtə bˈe", 39032, 0xce5a33aa2afd1434, 0xa1d7e29e46773f74, true),
    ("da", "00. b", false, "n'Ol n'Ol b'e", "nˈɒl nˈɒl bˈe", 41438, 0xec3f928d420f93f2, 0x18282254ea5908f5, true),
    ("no", "10:30.. b", false, "t'i: tR'Et:i:;@nda# b'e:", "tˈiː trˈɛtːiːənda bˈeː", 66032, 0x336e6ae147576466, 0x0, false),
    ("no", "5.\nb", false, "f'Em\nb'e:", "fˈɛm\nbˈeː", 52372, 0x42f1118eab3aee72, 0x0, false),
    ("no", "5.. b", false, "f'Emta# b'e:", "fˈɛmta bˈeː", 42194, 0xff16f7cb9987d07f, 0x0, false),
    ("no", "00. b", false, "n'u-l n'u-l b'e:", "nˈʉl nˈʉl bˈeː", 44696, 0xe22309fe0acd22e7, 0x0, false),
    ("de", "<speak>5.<mark name=\"m\"/> b</speak>", true, "f'ynft@ b'e:", "fˈʏnftə bˈeː", 45848, 0x57ccd50837eb8443, 0x0, false),
    ("de", "<speak>5.<emphasis>b</emphasis></speak>", true, "f,ynft@_! b'e:", "fˌʏnftə bˈeː", 45910, 0x04d55e33da95ba6f, 0x0, false),
    ("de", "<speak>10:30.<mark name=\"m\"/> b</speak>", true, "ts'e:n dr'aIsICst@ b'e:", "tsˈeːn dɾˈaɪsɪçstə bˈeː", 70044, 0x3bd43a43d46999ee, 0x0, false),
    ("nl", "<speak>5.<mark name=\"m\"/> b</speak>", true, "v'EIvd@ b'e:", "vˈɛɪvdə bˈeː", 46036, 0xb8ec2dfd5ba035c5, 0x0, false),
    ("nl", "<speak>5.<emphasis>b</emphasis></speak>", true, "v,EIvd@_! b'e:", "vˌɛɪvdə bˈeː", 45124, 0x8fc5ea4256ed7fec, 0x0, false),
    ("hu", "<speak>5.<mark name=\"m\"/> b</speak>", true, "_! 'YtYdik b'e:", "ˈøtødik bˈeː", 43444, 0xd7e42b2b2bcdaf8a, 0x0, false),
    ("hu", "<speak>10:30.<mark name=\"m\"/> b</speak>", true, "t'i:z h'AR2_mintsAdik b'e:", "tˈiːz hˈɑrmintsɑdik bˈeː", 73290, 0x7fda3f7e4e921574, 0x0, false),
    ("da", "<speak>5.<mark name=\"m\"/> b</speak>", true, "f'Emt@- b'e", "fˈɛmtə bˈe", 39340, 0x549ce6989d7be874, 0x0, false),
    ("no", "<speak>5.<emphasis>b</emphasis></speak>", true, "f,Emta#_! b'e:", "fˌɛmta bˈeː", 44100, 0x9f64229f6a447cf0, 0x0, false),
    ("en", "02:3\u{a0}b", false, "z'i@roU t'u: k'oUl@n Tr'i: b'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː bˈiː", 77890, 0x7c966e12fb205d77, 0xa8aac5cea3dda36e, true),
    ("de", "02:3\u{a0}b", false, "n'Ul tsv'aI dr'aI b'e:", "nˈʊl tsvˈaɪ dɾˈaɪ bˈeː", 64352, 0xee4581b9674af638, 0x0f0c0432c0694d3f, true),
    ("nl", "02:3\u{a0}b", false, "n'8l tv#,e: dr'i b'e:", "nˈɵl tʋˌeː drˈi bˈeː", 58726, 0xc39a5c0a90804d7a, 0x0035a7302e7be194, true),
    ("hu", "02:3\u{a0}b", false, "n'ullA kEt:Y: h'a:R2om b'e:", "nˈullɑ kɛtːøː hˈaːrom bˈeː", 74778, 0xdb9122f879927f4a, 0x8aae0afd96cd7fe9, true),
    ("da", "02:3\u{a0}b", false, "n'Ol t'o tR'?e b'e", "nˈɒl tˈo tʁˈʔe bˈe", 51712, 0x3ee4054a643d368e, 0xccf773b9002c16a0, true),
    ("nb", "02:3\u{a0}b", false, "n'u-l t'u: tR'e: b'e:", "nˈʉl tˈuː trˈeː bˈeː", 59942, 0x6c00970647b80b5b, 0x0, false),
    ("fr", "02:3\u{a0}b", false, "zero d'Y trw'a b'e", "zeʁo dˈø tʁwˈa bˈe", 52112, 0xc16911721fec45a5, 0x0dded32b43ae6886, true),
    ("es", "02:3\u{a0}b", false, "T'e**o D'os t**'es B'e", "θˈeɾo ðˈos tɾˈes βˈe", 56170, 0x7cb0e13813a1ccdc, 0xb4fa2565080fe8b8, true),
    ("ru", "02:3\u{a0}b", false, "n'ol dv'A tR;'i (en)b'i:(ru)", "nˈoɭ dvˈɑ trʲˈi (en)bˈiː(ru)", 52478, 0x14448c0e4b01cbf8, 0x6db735f508667cb7, true),
    ("pl", "02:3\u{a0}b", false, "z'ERO: dv'a tS'y b'E", "zˈɛrɔː dvˈa tʃˈɨ bˈɛ", 61002, 0x37a1f0436e266a5d, 0x94cb382e70006831, true),
    ("cs", "02:3\u{a0}b", false, "n'ula dv'a tR^'i b'e:", "nˈula dvˈa tr̝̊ˈi bˈeː", 53918, 0xa8384d0bc9593252, 0x5ebf2a135ed19e29, true),
    ("fi", "02:3\u{a0}b", false, "n'olla k'aksi k'olme b'e:", "nˈolla kˈaksi kˈolme bˈeː", 79236, 0x1a54c4f18f21e226, 0x89902b60a187431e, true),
    ("sv", "02:3\u{a0}b", false, "n'Ol tv'o: tR'e: b'e:", "nˈɔl tvˈoː trˈeː bˈeː", 37582, 0x9cf7238b116b6a43, 0x4b42c6557561906b, true),
    ("tr", "02:3\u{a0}b", false, "s@f'@R ic'i 'ytS b'e", "sɯfˈɯr icˈi ˈytʃ bˈe", 66702, 0xf8e519560fba6bf4, 0x9cbbaaad50ef6ffa, true),
    ("ja", "02:3\u{a0}b", false, "re'i_! ni:_! s'an_! _:(en)b'i:(ja)", "ɹe̞ˈi niː sˈän (en)bˈiː(ja)", 63252, 0x1ef665d4b0508e09, 0x0, false),
    ("uk", "02:3\u{a0}b", false, "n'ul^ dB'a dB'i: kR'apkI tR'I (en)b'i:(uk)", "nˈuʎ dβˈa dβˈiː krˈapkɪ trˈɪ (en)bˈiː(uk)", 124594, 0xc66c674f1043c437, 0xc962a192d7630d5a, true),
    ("it", "02:3\u{a0}b", false, "dz'E*o d'ue t@-*'e b'i", "dzˈɛɾo dˈue trˈe bˈi", 56340, 0xa80e735b493beea2, 0x1e35909649636978, true),
    ("pt", "02:3\u{a0}b", false, "z'E*u d'oIs# tR'eZ b'e", "zˈɛɾu dˈoɪʃ tɹˈeʒ bˈe", 62750, 0xeaf04543e52cd9bf, 0x7789852166f98b31, true),
    ("en", "02:3\u{202f}b", false, "z'i@roU t'u: k'oUl@n Tr'i: b'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː bˈiː", 77890, 0x7c966e12fb205d77, 0xa8aac5cea3dda36e, true),
    ("en", "02:3  b", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0x8fe391369ec26a68, true),
    ("en", "02:3 B", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0xfb56b404ee3eefee, true),
    ("en", "02:3\u{2002}b", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0xf77ce3ffec4d324d, true),
    ("en", "02:3\u{3000}b", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0xf77ce3ffec4d324d, true),
    ("en", "02:3\tb", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0xfb56b404ee3eefee, true),
    ("en", "02:3\nb", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0xfb56b404ee3eefee, true),
    ("en", "02:3 \u{a0}b", false, "z'i@roU t'u: k'oUl@n Tr'i: b'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː bˈiː", 77890, 0x7c966e12fb205d77, 0x0a5e289cb63cd51a, true),
    ("en", "02:3 'b", false, "t'u: k'oUl@n Tr'i:_:_: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 66622, 0xd25c6f8a258400c6, 0x407b6e8147316273, true),
    ("en", "02:3 _b", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0x11377d2aa3598d02, true),
    ("en", "02:3_b", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0x103dd369a5a016a6, true),
    ("en", "02:3.5", false, "z'i@roU t'u: k'oUl@n Tr'i: pOInt f'aIv", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː pɔɪnt fˈaɪv", 95784, 0xe37ff6cc55337fe4, 0x17477fbd7f39a164, true),
    ("en", "02:3,5 b", false, "t'u: k'oUl@n Tr'i: f'aIv b'i:", "tˈuː kˈəʊlən θɹˈiː fˈaɪv bˈiː", 75728, 0xa198b35253bacea2, 0x3fc6a3daf63d58fb, true),
    ("en", "<speak>02:3 b</speak>", true, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64428, 0x4ce7054429278195, 0x0, false),
    ("en", "<speak>02:3<mark name=\"m\"/> b</speak>", true, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64428, 0x4ce7054429278195, 0x0, false),
    ("en", "<speak>02:3\u{a0}b</speak>", true, "z'i@roU t'u: k'oUl@n Tr'i: b'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː bˈiː", 78198, 0x39a3bf5625b185a7, 0x0, false),
    ("de", "02:3\u{202f}b", false, "n'Ul tsv'aI dr'aI b'e:", "nˈʊl tsvˈaɪ dɾˈaɪ bˈeː", 64352, 0xee4581b9674af638, 0x0f0c0432c0694d3f, true),
    ("de", "02:3  b", false, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53374, 0x81959187cf3f1920, 0x185ec439dc4e719c, true),
    ("de", "02:3 B", false, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53374, 0x81959187cf3f1920, 0xcfe7e55a4231ceca, true),
    ("de", "02:3\u{2002}b", false, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53374, 0x81959187cf3f1920, 0x96d6c09238ecf04f, true),
    ("de", "02:3\u{3000}b", false, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53374, 0x81959187cf3f1920, 0x96d6c09238ecf04f, true),
    ("de", "02:3\tb", false, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53374, 0x81959187cf3f1920, 0xcfe7e55a4231ceca, true),
    ("de", "02:3\nb", false, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53374, 0x81959187cf3f1920, 0xcfe7e55a4231ceca, true),
    ("de", "02:3 \u{a0}b", false, "n'Ul tsv'aI dr'aI b'e:", "nˈʊl tsvˈaɪ dɾˈaɪ bˈeː", 64352, 0xee4581b9674af638, 0x74b9f724fa75ff81, true),
    ("de", "02:3 'b", false, "tsv'aI dr'aI_:_: b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 58708, 0xa84240396995542a, 0x0b06f487fe18f204, true),
    ("de", "02:3 _b", false, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53374, 0x81959187cf3f1920, 0x3bd0bbff04faae16, true),
    ("de", "02:3_b", false, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53374, 0x81959187cf3f1920, 0x65c7cb16dced37d2, true),
    ("de", "02:3.5", false, "tsv'aI dr'aI pUNkt f'ynf", "tsvˈaɪ dɾˈaɪ pʊŋkt fˈʏnf", 72240, 0x3fbd3b259df997e9, 0xea59d9f70e064f12, true),
    ("de", "02:3,5 b", false, "n'Ul tsv'aI dr'aI _kOmA: f'ynf b'e:", "nˈʊl tsvˈaɪ dɾˈaɪ kɔmɑː fˈʏnf bˈeː", 89968, 0x414b99deb30e5c2f, 0xc7a6d608ed89188a, true),
    ("de", "<speak>02:3 b</speak>", true, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53682, 0xa14042d065609320, 0x0, false),
    ("de", "<speak>02:3<mark name=\"m\"/> b</speak>", true, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53682, 0xa14042d065609320, 0x0, false),
    ("de", "<speak>02:3\u{a0}b</speak>", true, "n'Ul tsv'aI dr'aI b'e:", "nˈʊl tsvˈaɪ dɾˈaɪ bˈeː", 64660, 0x319a76b11c7bffb8, 0x0, false),
    ("fr", "02:3\u{202f}b", false, "zero d'Y trw'a b'e", "zeʁo dˈø tʁwˈa bˈe", 52112, 0xc16911721fec45a5, 0x0dded32b43ae6886, true),
    ("fr", "02:3  b", false, "d'Y trw'a b'e", "dˈø tʁwˈa bˈe", 38608, 0x83032102efadac1b, 0xd796e5b506a2c5c2, true),
    ("fr", "02:3\u{2002}b", false, "d'Y trw'a b'e", "dˈø tʁwˈa bˈe", 38608, 0x83032102efadac1b, 0x78d9666e96f0c03f, true),
    ("fr", "02:3 \u{a0}b", false, "zero d'Y trw'a b'e", "zeʁo dˈø tʁwˈa bˈe", 52112, 0xc16911721fec45a5, 0xb04b6786c49e17c0, true),
    ("fr", "02:3 'b", false, "d'Y trw'a_:_: b'e", "dˈø tʁwˈa bˈe", 41636, 0x4f2efd3866bc5376, 0x8eb6e250ab504a13, true),
    ("fr", "02:3,5 b", false, "zero d'Y trwa virgyl s'E~k b'e", "zeʁo dˈø tʁwa viʁɡyl sˈɛ̃k bˈe", 79724, 0xf2dcb2f3dee192bf, 0x9ee51489a834c1c6, true),
    ("fr", "<speak>02:3<mark name=\"m\"/> b</speak>", true, "d'Y trw'a b'e", "dˈø tʁwˈa bˈe", 38916, 0xb9374f26ce1e1b8b, 0x0, false),
    ("fr", "<speak>02:3\u{a0}b</speak>", true, "zero d'Y trw'a b'e", "zeʁo dˈø tʁwˈa bˈe", 52420, 0x8e199ddcb2f82235, 0x0, false),
    ("ru", "02:3\u{202f}b", false, "n'ol dv'A tR;'i (en)b'i:(ru)", "nˈoɭ dvˈɑ trʲˈi (en)bˈiː(ru)", 52478, 0x14448c0e4b01cbf8, 0x6db735f508667cb7, true),
    ("ru", "02:3  b", false, "dv'A tR;'i (en)b'i:(ru)", "dvˈɑ trʲˈi (en)bˈiː(ru)", 44682, 0x7f0a428b1bf14b53, 0x4a55eb0ed03e4e2d, true),
    ("ru", "02:3\u{3000}b", false, "dv'A tR;'i (en)b'i:(ru)", "dvˈɑ trʲˈi (en)bˈiː(ru)", 44682, 0x7f0a428b1bf14b53, 0x33f4392b0b601e4a, true),
    ("ru", "02:3 'b", false, "dv'A tR;'i_:_: (en)b'i:(ru)", "dvˈɑ trʲˈi (en)bˈiː(ru)", 49530, 0xfebf314f41b99393, 0x720a054ee303d79f, true),
    ("ru", "02:3.5", false, "dv'A tR;'i t'otS;ka# p;'At;", "dvˈɑ trʲˈi tˈotʃʲka pʲˈɑtʲ", 60266, 0x854ef3d36afdb5e1, 0x58270335d459a489, true),
    ("ru", "<speak>02:3<mark name=\"m\"/> b</speak>", true, "dv'A tR;'i (en)b'i:(ru)", "dvˈɑ trʲˈi (en)bˈiː(ru)", 44990, 0x7ec435d843f49443, 0x0, false),
    ("ru", "<speak>02:3\u{a0}b</speak>", true, "n'ol dv'A tR;'i (en)b'i:(ru)", "nˈoɭ dvˈɑ trʲˈi (en)bˈiː(ru)", 52786, 0x37c39fe8d0aa6178, 0x0, false),
    ("ja", "<speak>02:3 b</speak>", true, "n'i:_! s'an_! _:(en)b'i:(ja)", "nˈiː sˈän (en)bˈiː(ja)", 55006, 0x167acadaec10516d, 0x0, false),
    ("ja", "<speak>02:3<mark name=\"m\"/> b</speak>", true, "n'i:_! s'an_! _:(en)b'i:(ja)", "nˈiː sˈän (en)bˈiː(ja)", 55006, 0x167acadaec10516d, 0x0, false),
    ("ja", "<speak>02:3\u{a0}b</speak>", true, "re'i_! ni:_! s'an_! _:(en)b'i:(ja)", "ɹe̞ˈi niː sˈän (en)bˈiː(ja)", 63560, 0x7c42944aaff27dd9, 0x0, false),
    ("de", "002. b", false, "n'Uln'Ul tsv'aI b'e:", "nˈʊlnˈʊl tsvˈaɪ bˈeː", 61052, 0x94eb6058da837247, 0xb35e9585e908eb6f, true),
    ("nl", "002. b", false, "n'8ln,8l tv#,e: b'e:", "nˈɵlnˌɵl tʋˌeː bˈeː", 56172, 0x3caa234176af277c, 0xaf340e6cbabfbd35, true),
    ("hu", "002. b", false, "n'ullAnullA kEt:Y: b'e:", "nˈullɑnullɑ kɛtːøː bˈeː", 67576, 0x5f9961b4e5571cf0, 0x729772aa6ce742ca, true),
    ("nb", "002. b", false, "n'u-ln'u-l t'u: b'e:", "nˈʉlnˈʉl tˈuː bˈeː", 53174, 0x68aa24e3094bd1b5, 0x0, false),
    ("en", "02:3 _", false, "z'i@roU t'u: k'oUl@n Tr'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː", 67402, 0xcd915f8062e1b7d2, 0x737d6ea75825c920, true),
    ("en", "02:3_", false, "z'i@roU t'u: k'oUl@n Tr'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː", 67402, 0xcd915f8062e1b7d2, 0x72fa56fb0f8bf32d, true),
    ("en", "02:3 _5", false, "z'i@roU t'u: k'oUl@n Tr'i: f'aIv", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː fˈaɪv", 82404, 0x293118cd6e0b5edc, 0xe233a334e9eda890, true),
    ("en", "02:3_5", false, "z'i@roU t'u: k'oUl@n Tr'i: f'aIv", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː fˈaɪv", 82404, 0x293118cd6e0b5edc, 0x94bcff2f1a42b5d6, true),
    ("en", "02:3 _.", false, "z'i@roU t'u: k'oUl@n Tr'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː", 67402, 0xcd915f8062e1b7d2, 0x64c845eebeed08cb, true),
    ("en", "02:3 __", false, "z'i@roU t'u: k'oUl@n Tr'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː", 67402, 0xcd915f8062e1b7d2, 0x64c845eebeed08cb, true),
    ("en", "02:3 _, b", false, "z'i@roU t'u: k'oUl@n Tr'i:\nb'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː\nbˈiː", 85736, 0xb640551520a75601, 0xdc88890669426f58, true),
    ("en", "02:3 _ b", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0xc115d04a240da328, true),
    ("en", "02:3 __b", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0x4c3aceaa7be4d2be, true),
    ("en", "02:3 _'b", false, "t'u: k'oUl@n Tr'i:_:_: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 66622, 0xd25c6f8a258400c6, 0x5aef76853aa1d933, true),
    ("en", "02:3 ' b", false, "t'u: k'oUl@n Tr'i:_:_: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 66622, 0xd25c6f8a258400c6, 0x1297ba32b1290489, true),
    ("en", "02:3 '' b", false, "t'u: k'oUl@n Tr'i:_:_: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 66622, 0xd25c6f8a258400c6, 0xebc28c9878226e6f, true),
    ("en", "02:3 '_b", false, "t'u: k'oUl@n Tr'i:_:_: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 66622, 0xd25c6f8a258400c6, 0xf0c3dc038d3eb8cd, true),
    ("en", "02:3_ b", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0xbfc38e135bed0ffa, true),
    ("en", "02:3'b", false, "z'i@roU t'u: k'oUl@n Tr'i: b", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː b", 69880, 0x79e00c5dd6ff0d89, 0x757aa80bac59f036, true),
    ("en", "a,02:3 5 02:3 b", false, "a# z'i@roU t'u: k'oUl@n Tr'i: f'aIv t'u: k'oUl@n Tr'i: b'i:", "ɐ zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː fˈaɪv tˈuː kˈəʊlən θɹˈiː bˈiː", 131706, 0xdc0793b04702226a, 0x83983c78ae37e99a, true),
    ("en", "x:02:3 5 02:3 b", false, "'Eks k'oUl@n z'i@roU t'u: k'oUl@n Tr'i: f'aIv t'u: k'oUl@n Tr'i: b'i:", "ˈɛks kˈəʊlən zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː fˈaɪv tˈuː kˈəʊlən θɹˈiː bˈiː", 155836, 0xa1b6a312ab34e962, 0x8c50533f641b17e1, true),
    ("en", "1,02:3 5 02:3 b", false, "w'0n z'i@roU t'u: k'oUl@n Tr'i: f'aIv t'u: k'oUl@n Tr'i: b'i:", "wˈɒn zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː fˈaɪv tˈuː kˈəʊlən θɹˈiː bˈiː", 137306, 0x3150d47212ffe90f, 0xd543a2e68f54fa42, true),
    ("en", "a:02:3 b", false, "a# k'oUl@n t'u: k'oUl@n Tr'i: b'i:", "ɐ kˈəʊlən tˈuː kˈəʊlən θɹˈiː bˈiː", 84060, 0x0ce7dd57f070c89b, 0xab41bed88a43db3b, true),
    ("en", "02 :3 b", false, "t'u: k'oUl@n Tr'i: b'i:", "tˈuː kˈəʊlən θɹˈiː bˈiː", 64120, 0xd44232581a0b5705, 0xda7fb7baa1fdf8aa, true),
    ("en", "<speak><say-as interpret-as=\"characters\">02:3</say-as> b 02:3 5</speak>", true, "z,i@roU_|t,u:_|k,oUl@n_!Tr'i:_! b'i: z'i@roU t'u: k'oUl@n Tr'i: f'aIv", "zˌiəɹəʊtˌuːkˌəʊlənθɹˈiː bˈiː zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː fˈaɪv", 141408, 0x0ad205829175b0de, 0x0, false),
    ("en", "<speak><say-as interpret-as=\"characters\">02:3</say-as> b 02:3</speak>", true, "z,i@roU_|t,u:_|k,oUl@n_!Tr'i:_! b'i: z'i@roU t'u: k'oUl@n Tr'i:", "zˌiəɹəʊtˌuːkˌəʊlənθɹˈiː bˈiː zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː", 126544, 0x1c87f95bd1c4c6ac, 0x0, false),
    ("en", "<speak><say-as interpret-as=\"characters\">02:3 b</say-as> 02:3</speak>", true, "z,i@roU_|t,u:_|k,oUl@n_!Tr'i:_!_:_: b'i:_! z'i@roU t'u: k'oUl@n Tr'i:", "zˌiəɹəʊtˌuːkˌəʊlənθɹˈiː bˈiː zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː", 133578, 0x6ad5a22aa1c48f24, 0x0, false),
    ("en", "<speak>02:3 <say-as interpret-as=\"characters\">b</say-as> 02:3</speak>", true, "z'i@roU t'u: k'oUl@n Tr'i:_:_: b'i:_! z'i@roU t'u: k'oUl@n Tr'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː bˈiː zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː", 133762, 0x38eb3714721ecf79, 0x0, false),
    ("en", "<speak>02:3 <say-as interpret-as=\"characters\">b</say-as></speak>", true, "z'i@roU t'u: k'oUl@n Tr'i:_:_: b'i:_!", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː bˈiː", 82990, 0xc372afa27c3c95cf, 0x0, false),
    ("de", "02:3 _", false, "n'Ul tsv'aI dr'aI", "nˈʊl tsvˈaɪ dɾˈaɪ", 55000, 0x8c6726c108afc524, 0x40f521a1223e8cf8, true),
    ("de", "02:3 _, b", false, "n'Ul tsv'aI dr'aI\nb'e:", "nˈʊl tsvˈaɪ dɾˈaɪ\nbˈeː", 73400, 0x799cef8eb100f451, 0x5848dcc29aa0ee71, true),
    ("de", "02:3 _ b", false, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53374, 0x81959187cf3f1920, 0x4b71450dc3fa98b8, true),
    ("de", "02:3 ' b", false, "tsv'aI dr'aI_:_: b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 58708, 0xa84240396995542a, 0xf67b736c01f4c286, true),
    ("de", "a,02:3 5 02:3 b", false, "_!'A: k,Oman'Ultsv'aI_! dr'aI f'ynf tsv'aI dr'aI b'e:", "ˈɑː kˌɔmanˈʊltsvˈaɪ dɾˈaɪ fˈʏnf tsvˈaɪ dɾˈaɪ bˈeː", 118576, 0xb6dc77161375459c, 0xa4507d212fc7e4f8, true),
    ("de", "a.02:3 5 02:3 b", false, "_!'A: n'Ul tsv'aI dr'aI f'ynf tsv'aI dr'aI b'e:", "ˈɑː nˈʊl tsvˈaɪ dɾˈaɪ fˈʏnf tsvˈaɪ dɾˈaɪ bˈeː", 110556, 0xbedc6ce95f3cfbcf, 0xde02dbb1fd54c497, true),
    ("de", "1.02:3 5 02:3 b", false, "_'aIns pUNkt n'Ul tsv'aI dr'aI f'ynf tsv'aI dr'aI b'e:", "ˈaɪns pʊŋkt nˈʊl tsvˈaɪ dɾˈaɪ fˈʏnf tsvˈaɪ dɾˈaɪ bˈeː", 130242, 0xd0ea8e2968606180, 0xd316423ae585ce47, true),
    ("de", "02 :3 b", false, "tsv'aI dr'aI b'e:", "tsvˈaɪ dɾˈaɪ bˈeː", 53374, 0x81959187cf3f1920, 0xd7233513028cbcdd, true),
    ("de", "<speak><say-as interpret-as=\"characters\">02:3</say-as> b 02:3 5</speak>", true, "n,Ul_|tsv,aI_|d,Op@lp,UNkt_!dr'aI_! b'e: n'Ul tsv'aI dr'aI f'ynf", "nˌʊltsvˌaɪdˌɔpəlpˌʊŋktdɾˈaɪ bˈeː nˈʊl tsvˈaɪ dɾˈaɪ fˈʏnf", 136448, 0xcf22950da7d791ab, 0x0, false),
    ("de", "<speak>02:3 <say-as interpret-as=\"characters\">b</say-as> 02:3</speak>", true, "n'Ul tsv'aI dr'aI_:_: b'e:_! n'Ul tsv'aI dr'aI", "nˈʊl tsvˈaɪ dɾˈaɪ bˈeː nˈʊl tsvˈaɪ dɾˈaɪ", 110290, 0x57d9261d65f3c16f, 0x0, false),
    ("de", "5. \u{1}+2P b", false, "f'ynf\nb'e:", "fˈʏnf\nbˈeː", 55778, 0x45d574abee2b51a4, 0x50914af671e78f0a, true),
    ("de", "5. \u{1}30S b", false, "f'ynf\nb'e:", "fˈʏnf\nbˈeː", 98066, 0xbe93d4a3746e52d7, 0x9a92e796839fe766, true),
    ("de", "5.  \u{1}30S b", false, "f'ynf\nb'e:", "fˈʏnf\nbˈeː", 98066, 0xbe93d4a3746e52d7, 0xe6c165289d84261f, true),
    ("de", "5. \u{1}30S\u{1}20A b", false, "f'ynf\nb'e:", "fˈʏnf\nbˈeː", 99036, 0x24940b82b46cdcf7, 0xa410700842b8e045, true),
    ("de", "5. \u{1}30Sb", false, "f'ynf\nb'e:", "fˈʏnf\nbˈeː", 98066, 0xbe93d4a3746e52d7, 0xfa36bf3a0308ba38, true),
    ("hu", "5. \u{1}+2P b", false, "_! 'Yt\nb'e:", "ˈøt\nbˈeː", 45484, 0xbbce2a6d59c76985, 0x74e2bc02982c072c, true),
    ("hu", "5. \u{1}30S b", false, "_! 'Yt\nb'e:", "ˈøt\nbˈeː", 84434, 0x9c6b2c769e17da60, 0xb02dafd3dae6f916, true),
    ("fi", "5. \u{1}+2P b", false, "v'i:si\nb'e:", "vˈiːsi\nbˈeː", 65738, 0x7a748284f658aa90, 0x6511aa05d5acecdf, true),
    ("fi", "5. \u{1}30S b", false, "v'i:si\nb'e:", "vˈiːsi\nbˈeː", 116606, 0x1d6d788e39a39866, 0xe5697cb7db6d4dc5, true),
    ("fi", "1 000.. b", false, "t'uhat b'e:", "tˈuhat bˈeː", 48216, 0xdfee9766d2e49ef3, 0xe34c4482929533e6, true),
    ("fi", "1 005.. b", false, "t'uhat_! v'i:si b'e:", "tˈuhat vˈiːsi bˈeː", 66104, 0x9e9e8932fd75271d, 0x21f5c0c2829c9a7e, true),
    ("fi", "12 345.. b", false, "k'aksit,oista t,uhat:a_! k'olmes,ata:_ n,elj&kymment&v'i:si b'e:", "kˈaksitˌoista tˌuhatːa kˈolmesˌataː nˌeljækymmentævˈiːsi bˈeː", 171866, 0x8b3c3380cae18d30, 0x76f5af533464ff69, true),
    ("et", "1 000.. b", false, "t'uhA1t b'ee:", "tˈuhɑt bˈeeː", 49586, 0x49b49dace4e2b5e8, 0xf8e6ae0d7f1b5385, true),
    ("et", "12 345.. b", false, "k'akste,ist tuhA1t_! k'ollmsadA1_ n'elI1 k,ymmendv'ii:s b'ee:", "kˈaksteˌist tuhɑt kˈollmsadɑ nˈelɪ kˌymmendvˈiiːs bˈeeː", 155312, 0x1d305fa8b01e7e35, 0x533eda0f9d4bfa34, true),
    ("sl", "1 005.. b", false, "t'i:sOtS_! p'e:d b'@", "tˈiːsɔtʃ pˈeːd bˈə", 61528, 0x3d5152120170279d, 0x58344d27dd4ed005, true),
    ("fi", "1 000.  b", false, "t'uhat b'e:", "tˈuhat bˈeː", 48216, 0xdfee9766d2e49ef3, 0xe34c4482929533e6, true),
    ("fi", "12 345.  b", false, "k'aksit,oista t,uhat:a_! k'olmes,ata:_ n,elj&kymment&v'i:si b'e:", "kˈaksitˌoista tˌuhatːa kˈolmesˌataː nˌeljækymmentævˈiːsi bˈeː", 171866, 0x8b3c3380cae18d30, 0x76f5af533464ff69, true),
    ("fi", "99 999.  maj", false, ",yhdeks,&Nkymment&'yhdeks&n t,uhat:a_! 'yhdeks&ns,ata:_ ,yhdeks,&Nkymment&'yhdeks&n m'aj", "ˌyhdeksˌæŋkymmentæˈyhdeksæn tˌuhatːa ˈyhdeksænsˌataː ˌyhdeksˌæŋkymmentæˈyhdeksæn mˈaj", 223968, 0xc06fce11bc9321cb, 0x650c86ad17b4c93e, true),
    ("fi", "1 000 000.  kello", false, "m'iljo:na k'ello", "mˈiljoːna kˈello", 54258, 0x4250ebd1e1a49707, 0x5a244ef340b420ed, true),
    ("de", "1 000.  b", false, "_'aIn t'aUz@nt b'e:", "ˈaɪn tˈaʊzənt bˈeː", 56662, 0xa788a6a33939e829, 0x435698342d68d357, true),
    ("nl", "1 000.  b", false, "d'WyzEnt b'e:", "dˈœyzɛnt bˈeː", 45360, 0x79c09d040b5aca2e, 0x8ddf41e8b8077e3, true),
    ("hu", "1 000.  b", false, "_! 'EzER2 b'e:", "ˈɛzɛr bˈeː", 37824, 0x84ccb8e2a9bcc684, 0x2b04443d42bbd1fe, true),
    ("nb", "1 000.  b", false, "'Et t'u-:s@n b'e:", "ˈɛt tˈʉːsən bˈeː", 52012, 0xcd5d7cc97da18e50, 0x0, false),
    ("et", "1 000.  b", false, "t'uhA1t b'ee:", "tˈuhɑt bˈeeː", 49586, 0x49b49dace4e2b5e8, 0xf8e6ae0d7f1b5385, true),
    ("et", "12 345.  b", false, "k'akste,ist tuhA1t_! k'ollmsadA1_ n'elI1 k,ymmendv'ii:s b'ee:", "kˈaksteˌist tuhɑt kˈollmsadɑ nˈelɪ kˌymmendvˈiiːs bˈeeː", 155312, 0x1d305fa8b01e7e35, 0x533eda0f9d4bfa34, true),
    ("sl", "99 999.  maj", false, "dEv'e:tindEv'e:ddEsEt t'i:sOtS_! dEv'e:tstO dEv'e:tindEv'e:ddEsEt m'a:j", "dɛʋˈeːtindɛʋˈeːddɛsɛt tˈiːsɔtʃ dɛʋˈeːtstɔ dɛʋˈeːtindɛʋˈeːddɛsɛt mˈaːj", 177722, 0xfefc1c51c8ffbc13, 0x7fd64246ea2d242d, true),
    ("sl", "1 000 000.  kello", false, "'Enmilij'o:n k'e:wlO", "ˈɛnmilijˈoːn kˈeːwlɔ", 61648, 0xd72d30f0d5862de8, 0xa434682dc41e521b, true),
    ("nb", "5.. 日", false, "f'Em\n_:(en)tS'aIni:z(no)l'et@", "fˈɛm\n(en)tʃˈaɪniːz(no)lˈetə", 75604, 0x208cdbcb3392ff43, 0x0, false),
    ("nl", "5.  日", false, "v'EIf\n_:(en)tS'aIni:z(nl)l'Et@r", "vˈɛɪf\n(en)tʃˈaɪniːz(nl)lˈɛtər", 81778, 0xe427f5753828174e, 0xc5d90a0bee331370, true),
    ("no", "31.  日", false, "tR'Et:i:;'e:n\n_:(en)tS'aIni:z(no)l'et@", "trˈɛtːiːˈeːn\n(en)tʃˈaɪniːz(no)lˈetə", 95908, 0x6e8bcdae1ece3a8a, 0x0, false),
    ("nl", "23:9.  日", false, "dr'iEntv#,Int@x n'e:Q@n\n_:(en)tS'aIni:z(nl)l'Et@r", "drˈiɛntʋˌɪntəx nˈeːɣən\n(en)tʃˈaɪniːz(nl)lˈɛtər", 115306, 0xf6bdd4725a754f26, 0x839dfb3163deaf30, true),
    ("hu", "a 31.. 日 123 456 789. 5", false, "_|,A h'AR2mintsEJ\nk'i:nAikAR2AktER2 s'a:z husonha:R2om milli;o:_! n'e:Jsa:z YtvEnhAt EzER2_! h'e:tsa:z n^oltsvAnkilEnts\n_! 'Yt", "ˌɑ hˈɑrmintsɛɟ\nkˈiːnɑikɑrɑktɛr sˈaːz husonhaːrom millioː nˈeːɟsaːz øtvɛnhɑt ɛzɛr hˈeːtsaːz ɲoltsvɑnkilɛnts\n ˈøt", 321662, 0xc7c03a4ccc3007db, 0x434f15c21efa0a5d, true),
    ("de", "5. 日", false, "f'ynf\nCi:n'e:zISb'u:xStA:b@", "fˈʏnf\nçiːnˈeːzɪʃbˈuːxʃtɑːbə", 93834, 0x1f2f37d9cc7d27af, 0x9aae6be43464839f, true),
    ("de", "5.. 日", false, "f'ynf\nCi:n'e:zISb'u:xStA:b@", "fˈʏnf\nçiːnˈeːzɪʃbˈuːxʃtɑːbə", 93834, 0x1f2f37d9cc7d27af, 0x72eb5a775ed83ac5, true),
    ("fi", "5.  日", false, "v'i:si\n_:(en)tS'aIni:z(fi)m'erk:I", "vˈiːsi\n(en)tʃˈaɪniːz(fi)mˈerkːɪ", 91880, 0x24b0a2599b60a0c7, 0x103c135492378ee0, true),
    ("sv", "02:03:9 \u{1}100sb", false, "tv'o: n'Ol tR'e: n'i:;u: 'Eth'8ndRa ,Esb'e:", "tvˈoː nˈɔl trˈeː nˈiːuː ˈɛthˈɵndra ˌɛsbˈeː", 63728, 0xb7fab485bd9a4bda, 0xf8fd44be13ad90e8, true),
    ("tr", "0::03:5.\u{1}-3s5", false, "s@f'@R s@f'@R 'ytS b'ES\n'ytS s'E b'ES", "sɯfˈɯr sɯfˈɯr ˈytʃ bˈɛʃ\nˈytʃ sˈɛ bˈɛʃ", 123504, 0x327e5ab24958c078, 0xa5a0cf1ba419d078, true),
    ("en", "02:3 \u{1}100sb", false, "z'i@roU t'u: k'oUl@n Tr'i: w'0nh'VndrI2d ,Esb'i:", "zˈiəɹəʊ tˈuː kˈəʊlən θɹˈiː wˈɒnhˈʌndɹɪd ˌɛsbˈiː", 110284, 0xebe6390139708da6, 0x57979386be55a621, true),
    ("en", "<speak>00:0,\u{a0}B</speak>", true, "z'i@roU z'i@roU k'oUl@n z'i@roU b'i:", "zˈiəɹəʊ zˈiəɹəʊ kˈəʊlən zˈiəɹəʊ bˈiː", 89956, 0xbfaf91070329d9db, 0x0, false),
    ("es", "12:00:3,- mai", false, "d'oTe T'e**o T'e**o t**'es m'aI", "dˈoθe θˈeɾo θˈeɾo tɾˈes mˈaɪ", 71442, 0x85a9922f71e5a3b5, 0xc7d3b9b4a879190e, true),
    ("tr", "<speak>10:03:9.- kello<break time=\"100ms\"/>b</speak>", true, "'on s@f'@R 'ytS d'okuz k&lL'O\nb'e", "ˈon sɯfˈɯr ˈytʃ dˈokuz kælɫˈɔ\nbˈe", 104518, 0x96e195ff1f8ad18f, 0x0, false),
    ("cs", "<speak>007:00:3, Uhr</speak>", true, "n'ulan'ula s'edum n'ula n'ula tR^'i\n'uhr-", "nˈulanˈula sˈedum nˈula nˈula tr̝̊ˈi\nˈuhr̩", 89456, 0x78191b23c91d11a2, 0x0, false),
    ("hr", "12:00::5,-\u{1}100sb", false, "dv'anaIzd dv'ototSk& n'ul& n'ul& dv'ototSk& p'Et st'o_ s'@b,@", "dvˈanaɪzd dvˈototʃkæ nˈulæ nˈulæ dvˈototʃkæ pˈɛt stˈo sˈəbˌə", 141008, 0xe1b539d32c2f615d, 0x0, false),
    ("da", "a 00:5,- und 007: 59_ \u{1}100smai", false, "'?& n'Ol n'Ol f'?Em '?On n'Oln'Ol s'?yw\nn'?iVh?&lt*'?Es ed h'unRV:D sm'?&?i", "ˈa nˈɒl nˈɒl fˈεm ˈɒn nˈɒlnˈɒl sˈʔyw\nnˈʔiʌhaltɾˈεs ed hˈunʁʌːð smˈaʔi", 156256, 0xbfe9559a39c02b01, 0x9041e62e4dfb72e1, true),
    ("fi", "00:3,-5", false, "n'olla n'olla k'olme v'i:si", "nˈolla nˈolla kˈolme vˈiːsi", 78924, 0xb09c35dab47cfcf2, 0x94088abeccf408ce, true),
    ("de", "8. سلام", false, "_'axt\n(ar)sal'a:m(de)", "ˈaxt\n(ar)salˈaːm(de)", 60960, 0xd055701111ae4ee6, 0xb48426cb270e5bd9, true),
    ("de", "8.  سلام", false, "_'axt\n(ar)sal'a:m(de)", "ˈaxt\n(ar)salˈaːm(de)", 60960, 0xd055701111ae4ee6, 0x100cabe595d10371, true),
    ("de", "8.. שלום", false, "_'axt\nhEbr'E:IS_:(he)S'in(de) hEbr'E:IS_:(he)l'amed(de) hEbr'E:IS_:(he)v'av(de) hEbr'E:IS_:(he)m'em(de)", "ˈaxt\nhɛbɾˈɛːɪʃ(he)ʃˈin(de) hɛbɾˈɛːɪʃ(he)lˈamed(de) hɛbɾˈɛːɪʃ(he)vˈav(de) hɛbɾˈɛːɪʃ(he)mˈem(de)", 196142, 0x50f9bb441de5707a, 0x0, false),
    ("de", "8. \u{3000}हिन्दी", false, "_'axt\n(hi)H'Indi(de)", "ˈaxt\n(hi)hˈɪndi(de)", 55108, 0x2a8565bdf6d0c826, 0x47f53e8228e61c2, true),
    ("lv", "1133.. שלום", false, "t'u:kstuo:tis s'imts tR'i:sdesmit tR'i:s\n_:(en)h'i:bru:_:(he)S'in _:(en)h'i:bru:_:(he)l'amed _:(en)h'i:bru:_:(he)v'av _:(en)h'i:bru:_:(he)m'em(lv)", "tˈuːkstuoːtis sˈimt͡s trˈiːsdesmit trˈiːs\n(en)hˈiːbɹuː(he)ʃˈin (en)hˈiːbɹuː(he)lˈamed (en)hˈiːbɹuː(he)vˈav (en)hˈiːbɹuː(he)mˈem(lv)", 255838, 0xe606710133ffd150, 0xb32cb3b809f2917f, true),
    ("lv", "866..\tسلام", false, "'astuon^i s'imti s'eSdesmit s'eSi\n(ar)sal'a:m(lv)", "ˈastuoɲi sˈimti sˈeʃdesmit sˈeʃi\n(ar)salˈaːm(lv)", 122182, 0x28e2fa19a924f0c6, 0xf6f85feb60e6b61f, true),
    ("nl", "godzina 760.742.. שלום", false, "QOdz'ina: z'e:v@nh,Ond@rt z,Est@x d,WyzEnt_! z'e:v@nh,Ond@r! tv#,e:Enf,I:rt@x\n_:(en)h'i:bru:_:(he)S'in _:(en)h'i:bru:_:(he)l'amed _:(en)h'i:bru:_:(he)v'av _:(en)h'i:bru:_:(he)m'em(nl)", "ɣɔdzˈinaː zˈeːvənhˌɔndərt zˌɛstəx dˌœyzɛnt zˈeːvənhˌɔndər tʋˌeːɛnfˌɪːrtəx\n(en)hˈiːbɹuː(he)ʃˈin (en)hˈiːbɹuː(he)lˈamed (en)hˈiːbɹuː(he)vˈav (en)hˈiːbɹuː(he)mˈem(nl)", 328452, 0xe3418954a4be39fb, 0x4e52a2782aeebab, true),
    ("da", "409 437. \u{3000}ไทย", false, "f'iVh'unRVDV n'?i f'iVh'unRVDV s'?ywVt*'?&#Dv@\n_:(en)t'aI(da)boustaw_|'e:_|f'iV_|f'iV_: _:(en)t'aI(da)boustaw_|'e:_|'e:n_|s'?yw_: _:(en)t'aI(da)boustaw_|'e:_|t'o_|t'o_:", "fˈiʌhˈunʁʌðʌ nˈʔi fˈiʌhˈunʁʌðʌ sˈʔywʌtɾˈ?aðʋə\n(en)tˈaɪ(da)boustawˈeːfˈiʌfˈiʌ (en)tˈaɪ(da)boustawˈeːˈeːnsˈʔyw (en)tˈaɪ(da)boustawˈeːtˈotˈo", 280654, 0x5161c664d7b4a630, 0x0, false),
    ("fi", "1419. \u{3000}שלום", false, "t'uhat n'elj&s,ata:_ 'yhdeks&nt,oista\n_:(en)h'i:bru:_:(he)S'in _:(en)h'i:bru:_:(he)l'amed _:(en)h'i:bru:_:(he)v'av _:(en)h'i:bru:_:(he)m'em(fi)", "tˈuhat nˈeljæsˌataː ˈyhdeksæntˌoista\n(en)hˈiːbɹuː(he)ʃˈin (en)hˈiːbɹuː(he)lˈamed (en)hˈiːbɹuː(he)vˈav (en)hˈiːbɹuː(he)mˈem(fi)", 244212, 0x191808f75198a356, 0xe945e2f2e5f851b0, true),
    ("fi", "2029. \tไทย", false, "k'aksi t,uhat:a k,aksikymment&'yhdeks&n\n_:(en)t'aI(fi)m'erk:I_|'e:_|n'elj&_|n'elj&_: _:(en)t'aI(fi)m'erk:I_|'e:_|'yksI_|s'eitsem&n_: _:(en)t'aI(fi)m'erk:I_|'e:_|k'aksI_|k'aksI_:", "kˈaksi tˌuhatːa kˌaksikymmentæˈyhdeksæn\n(en)tˈaɪ(fi)mˈerkːɪˈeːnˈeljænˈeljæ (en)tˈaɪ(fi)mˈerkːɪˈeːˈyksɪsˈeitsemæn (en)tˈaɪ(fi)mˈerkːɪˈeːkˈaksɪkˈaksɪ", 357526, 0x607e27ad6eac98c4, 0x0, false),
    ("nb", "αβγ\u{1}1b часов1094. \u{3000}ไทย", false, "(el)'avQ(no) 'e:n b'e: _:(en)sI2r'IlIk_:tS'E: _:sI2r'IlIk_:'a: _:sI2r'IlIk_:'Es _:sI2r'IlIk_:'o: _:sI2r'IlIk_:v'E:(no) 'Et t'u-:s@n u:g n'It:i:f'i:RE\n_:(en)t'aI(no)l'et@_|'e:_|f'i:RE_|f'i:RE_: _:(en)t'aI(no)l'et@_|'e:_|'e:n_|S'u-:_: _:(en)t'aI(no)l'et@_|'e:_|t'u:_|t'u:_:", "(el)ˈavɣ(no) ˈeːn bˈeː (en)sɪɹˈɪlɪktʃˈɛː sɪɹˈɪlɪkˈaː sɪɹˈɪlɪkˈɛs sɪɹˈɪlɪkˈoː sɪɹˈɪlɪkvˈɛː(no) ˈɛt tˈʉːsən uːɡ nˈɪtːiːfˈiːrɛ\n(en)tˈaɪ(no)lˈetəˈeːfˈiːrɛfˈiːrɛ (en)tˈaɪ(no)lˈetəˈeːˈeːnʃˈʉː (en)tˈaɪ(no)lˈetəˈeːtˈuːtˈuː", 440252, 0x73e67d27763c40e5, 0x0, false),
    ("de", "8. ä", false, "_'axt@ _!'E:", "ˈaxtə ˈɛː", 37432, 0x8bbf280e0fb9b4c7, 0xa69b047f2976189a, true),
    ("de", "8. б", false, "_'axt@ ky:R'IlIS_:(en)b'E:(de)", "ˈaxtə kyːrˈɪlɪʃ(en)bˈɛː(de)", 63546, 0x73f0d0b45269cf1e, 0x2c939ae55bc94cc9, true),
    ("fi", "а 1. ǲungla und so", false, "_:(en)sI2r'IlIk_:'a:(fi) 'yksi d'uNla 'und s'o", "(en)sɪɹˈɪlɪkˈaː(fi) ˈyksi dˈuŋla ˈund sˈo", 91926, 0x6dffdb6b847482cb, 0x0, false),
    ("fi", "On 100.\u{b}ǅλα. Ende", false, "on s'ata m'erk:I_|'yksI_|s'e:_|k'u:sI_: (el)l'a(fi)\n'ende", "on sˈata mˈerkːɪˈyksɪsˈeːkˈuːsɪ (el)lˈa(fi)\nˈende", 133794, 0xd7f89c2072dea164, 0x0, false),
    ("de", "8. ǅa", false, "_'axt d'A:", "ˈaxt dˈɑː", 37906, 0x154b0e693207a319, 0x0, false),
    ("de", "8. ǈa", false, "_'axt l'A:", "ˈaxt lˈɑː", 37774, 0x517379ebb6c3b5d5, 0x0, false),
    ("de", "8. ǋa", false, "_'axt n'A:", "ˈaxt nˈɑː", 39104, 0x6839b9e17bcf36fc, 0x0, false),
    ("de", "8. ǲa", false, "_'axt d'A:", "ˈaxt dˈɑː", 37906, 0x154b0e693207a319, 0x0, false),
    ("sl", "8. ǅa", false, "'o:s@m d'a:", "ˈoːsəm dˈaː", 43698, 0x6bcd34ecfa21bbc4, 0x0, false),
    ("de", "5.  ꟁabc", false, "f'ynf\n_!'apk", "fˈʏnf\nˈapk", 57214, 0xd4484c7a0f2eece6, 0x37832c4fedfb1841, true),
    ("de", "5.. 𝼀a", false, "f'ynf\n_!'A:", "fˈʏnf\nˈɑː", 55950, 0x13b1fef8abcd5532, 0xecc791fcdaac84af, true),
    ("de", "8. ᲊa", false, "_'axt\n_!'A:", "ˈaxt\nˈɑː", 51528, 0xa3bad3b984f6e45b, 0x97109503ef955c16, true),
    ("de", "8. ⱟa", false, "_'axt\n_!'A:", "ˈaxt\nˈɑː", 51528, 0xa3bad3b984f6e45b, 0x97109503ef955c16, true),
    ("de", "8. ä", false, "_'axt@ _!'E:", "ˈaxtə ˈɛː", 37432, 0x8bbf280e0fb9b4c7, 0xa69b047f2976189a, true),
];

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
        .filter(|&(_, same)| !same)
        .map(|(field, _)| format!("{voice} {field} {input:?}"))
        .collect()
}

#[test]
fn glued_commas_one_digit_minutes_and_ordinal_dots_read_as_c_reads_them() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = ROWS.iter().flat_map(differences).collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
