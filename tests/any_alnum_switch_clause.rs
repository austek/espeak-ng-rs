// tests/any_alnum_switch_clause.rs
//
// Two edges of the clause rule that C's `ReadClause` sets.  At a bare `\x01V` the reader drops the
// blank after it, so a mark after the voice change still counts as a mark that a blank follows.
// A word of another language that closes a clause ends its switch there: the clause that follows
// opens its own, so a clause end is no word gap between two switched words.  Expected values are
// what C espeak-ng master (f0fb11a8) produces: `-q -x`, `-q --ipa`, the PCM of `-w` (byte count
// and FNV-1a 64 hash) and the word/sentence/end events (FNV-1a 64 hash of the transcript lines
// `ev <pos> <len> <ms> <sample> <type> num=<n>` and `samples=<n>`).
//
// Each row's last field says which of the four this port reproduces.  The others differ from C
// for reasons outside the rule: the paragraph pause of `\n\n`, the word split of an English word
// in Japanese and the pause after a Mandarin clause.

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
    let dir = std::env::temp_dir().join(format!("any_alnum_switch_{}", std::process::id()));
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
    ("en", "? \u{1}V ! a", "_:'Ekskl@m,eIS@n_: 'eI", "ˈɛkskləmˌeɪʃən ˈeɪ", 62112, 0x4342d73b36349412, 0x74cb4227725d7e5c, X | IPA | WAV),
    ("en", "! \u{1}V ? a", "_:'Ekskl@m,eIS@n_:\n'eI", "ˈɛkskləmˌeɪʃən\nˈeɪ", 62512, 0x087dcdda4bbf1917, 0x1c8da0c538cd4a60, X | IPA | WAV),
    ("en", ". \u{1}V ! a", "_:'Ekskl@m,eIS@n_: 'eI", "ˈɛkskləmˌeɪʃən ˈeɪ", 62112, 0x4342d73b36349412, 0x4f560be9564010e9, X | IPA | WAV),
    ("en", "? \u{1}V . a", "'eI", "ˈeɪ", 23682, 0x8b111589865601bc, 0x65942ac030395cae, ALL),
    ("en", "a \u{1}V ! b", "'eI\n_:'Ekskl@m,eIS@n_: b'i:", "ˈeɪ\nˈɛkskləmˌeɪʃən bˈiː", 71328, 0x75defbfe175691e1, 0xd008ff6a352c2560, X | IPA | WAV),
    ("en", "a ! \u{1}V b", "'eI\n\nb'i:", "ˈeɪ\n\nbˈiː", 50620, 0xfcef2d86c2792993, 0x39420da709f3da9b, ALL),
    ("uk", "? \u{1}V ! a", "zn'ak 'okleku (en)'eI(uk)", "znˈak ˈokleku (en)ˈeɪ(uk)", 77584, 0x2a648095415437ea, 0x66b7c6e223ef5b56, X | IPA | WAV),
    ("uk", "! \u{1}V ? a", "zn'ak 'okleku\n(en)'eI(uk)", "znˈak ˈokleku\n(en)ˈeɪ(uk)", 78572, 0xd2ffab0436b483c4, 0x550f93c19bf8c3f0, X | IPA),
    ("uk", ". \u{1}V ! a", "zn'ak 'okleku (en)'eI(uk)", "znˈak ˈokleku (en)ˈeɪ(uk)", 77584, 0x2a648095415437ea, 0xde5b657933f6a416, X | IPA | WAV),
    ("uk", "? \u{1}V . a", "(en)'eI(uk)", "(en)ˈeɪ(uk)", 33802, 0x18ae6e4325ed9a03, 0x3351cecca6f7ddea, ALL),
    ("uk", "a \u{1}V ! b", "(en)'eI(uk)\nzn'ak 'okleku (en)b'i:(uk)", "(en)ˈeɪ(uk)\nznˈak ˈokleku (en)bˈiː(uk)", 92420, 0x34da4711014145df, 0xf818346c16192e98, X | IPA | WAV),
    ("uk", "a ! \u{1}V b", "(en)'eI(uk)\n\n(en)b'i:(uk)", "(en)ˈeɪ(uk)\n\n(en)bˈiː(uk)", 72332, 0xf594151eead5d11e, 0xd852155b4776ad2b, ALL),
    ("uk", "a\u{1}V. Ok", "(en)'eI(uk)\n(en),oUk'eI(uk)", "(en)ˈeɪ(uk)\n(en)ˌəʊkˈeɪ(uk)", 56950, 0x968d8ad3a75b7539, 0x564593cc93bf1704, X | IPA | WAV),
    ("uk", "Hi.\u{1}V ok", "(en)h'aI(uk)\n\n(en),oUk'eI(uk)", "(en)hˈaɪ(uk)\n\n(en)ˌəʊkˈeɪ(uk)", 83042, 0xab6d32df30e85896, 0x25a25df4af789a2a, ALL),
    ("uk", "Hi. \u{1}V ok", "(en)h'aI(uk)\n\n(en),oUk'eI(uk)", "(en)hˈaɪ(uk)\n\n(en)ˌəʊkˈeɪ(uk)", 83042, 0xab6d32df30e85896, 0xe36b3f55a75cc31d, ALL),
    ("uk", "a .\nb .\n\nc", "(en)'eI(uk)\n(en)b'i:(uk)\n(en)s'i:(uk)", "(en)ˈeɪ(uk)\n(en)bˈiː(uk)\n(en)sˈiː(uk)", 120426, 0x1ba45609ac9ad6b0, 0xd7b50e4c37a7d1e9, X | IPA),
    ("uk", "Hi. Ok", "(en)h'aI(uk)\n(en),oUk'eI(uk)", "(en)hˈaɪ(uk)\n(en)ˌəʊkˈeɪ(uk)", 83042, 0xab6d32df30e85896, 0xfa5e9f572a0d6fd5, ALL),
    ("uk", "a. Ok", "(en)'eI(uk)\n(en),oUk'eI(uk)", "(en)ˈeɪ(uk)\n(en)ˌəʊkˈeɪ(uk)", 78376, 0x440c30be424dd767, 0x8605acbc6c670db3, ALL),
    ("el", "? \u{1}V ! a", "T,avmastik'o (en)'eI(el)", "θˌavmastikˈo (en)ˈeɪ(el)", 54820, 0x1622780cdabedec3, 0x60471759ab2379db, X | IPA | WAV),
    ("el", "! \u{1}V ? a", "T,avmastik'o\n(en)'eI(el)", "θˌavmastikˈo\n(en)ˈeɪ(el)", 55584, 0x5de96e4cfdfc52c0, 0x3ec8fca4c601db74, X | IPA | WAV),
    ("el", ". \u{1}V ! a", "T,avmastik'o (en)'eI(el)", "θˌavmastikˈo (en)ˈeɪ(el)", 54820, 0x1622780cdabedec3, 0xdac5b5da62407c5e, X | IPA | WAV),
    ("el", "? \u{1}V . a", "(en)'eI(el)", "(en)ˈeɪ(el)", 23870, 0x2dbdea58ee673745, 0xf8ce2df8e78d46bd, ALL),
    ("el", "a \u{1}V ! b", "(en)'eI(el)\nT,avmastik'o (en)b'i:(el)", "(en)ˈeɪ(el)\nθˌavmastikˈo (en)bˈiː(el)", 64592, 0xd6bd794a55919a3c, 0x56e20d1ccf2af895, X | IPA | WAV),
    ("el", "a ! \u{1}V b", "(en)'eI(el)\n\n(en)b'i:(el)", "(en)ˈeɪ(el)\n\n(en)bˈiː(el)", 51032, 0x56f4c7c360ed9078, 0x553717e880971800, ALL),
    ("el", "a\u{1}V. Ok", "(en)'eI(el)\n(en),oUk'eI(el)", "(en)ˈeɪ(el)\n(en)ˌəʊkˈeɪ(el)", 40012, 0xff97aeb04fc298e0, 0x2948ddb938585f0d, X | IPA | WAV),
    ("el", "Hi.\u{1}V ok", "(en)h'aI(el)\n\n(en),oUk'eI(el)", "(en)hˈaɪ(el)\n\n(en)ˌəʊkˈeɪ(el)", 59132, 0x3ee981d4d842e259, 0xa513403f55e26b16, ALL),
    ("el", "Hi. \u{1}V ok", "(en)h'aI(el)\n\n(en),oUk'eI(el)", "(en)hˈaɪ(el)\n\n(en)ˌəʊkˈeɪ(el)", 59132, 0x3ee981d4d842e259, 0x06d7f05a330dc489, ALL),
    ("el", "a .\nb .\n\nc", "(en)'eI(el)\n(en)b'i:(el)\n(en)s'i:(el)", "(en)ˈeɪ(el)\n(en)bˈiː(el)\n(en)sˈiː(el)", 85160, 0x80b3f446a2a57cae, 0x9b8ac53df288f63b, X | IPA),
    ("el", "a ... b", "(en)'eI(el)\n(en)b'i:(el)", "(en)ˈeɪ(el)\n(en)bˈiː(el)", 45982, 0x6aca51bb878a5703, 0xcdcf4a4647c2d010, ALL),
    ("el", "Hi. Ok", "(en)h'aI(el)\n(en),oUk'eI(el)", "(en)hˈaɪ(el)\n(en)ˌəʊkˈeɪ(el)", 59132, 0x3ee981d4d842e259, 0x9c33da735de37acb, ALL),
    ("el", "a. Ok", "(en)'eI(el)\n(en),oUk'eI(el)", "(en)ˈeɪ(el)\n(en)ˌəʊkˈeɪ(el)", 55988, 0x55bac5a648cd9b31, 0xaf9f30e81f24c326, ALL),
    ("ja", "? \u{1}V ! a", "_:(en)'eI(ja)", "(en)ˈeɪ(ja)", 26920, 0xef5d7ce23e2bc0d8, 0xb4eb828e791548b4, ALL),
    ("ja", ". \u{1}V ! a", "_:(en)'eI(ja)", "(en)ˈeɪ(ja)", 26920, 0xef5d7ce23e2bc0d8, 0x739ed1a669a4b7a3, ALL),
    ("ja", "a \u{1}V ! b", "_:(en)'eI(ja)\n_:(en)b'i:(ja)", "(en)ˈeɪ(ja)\n(en)bˈiː(ja)", 40634, 0x071ae7b9aff3fdcc, 0x6c784507e6079a63, ALL),
    ("ja", "a ! \u{1}V b", "_:(en)'eI(ja)\n\n_:(en)b'i:(ja)", "(en)ˈeɪ(ja)\n\n(en)bˈiː(ja)", 58634, 0xc68ab2a57d6c82c8, 0x819cfede1499d9f5, ALL),
    ("ja", "a\u{1}V. Ok", "_:(en)'eI(ja)\n_:(en)'oU _:k'eI(ja)", "(en)ˈeɪ(ja)\n(en)ˈəʊ kˈeɪ(ja)", 53182, 0x03e1f9212395b1f3, 0x3386b83a2bd88596, X | IPA | WAV),
    ("ja", "Hi.\u{1}V ok", "_:(en)'eItS _:'aI(ja)\n\n_:(en)'oU _:k'eI(ja)", "(en)ˈeɪtʃ ˈaɪ(ja)\n\n(en)ˈəʊ kˈeɪ(ja)", 84122, 0xa42a8f59ba1dea04, 0x9195af6b248c8028, X | IPA | WAV),
    ("ja", "Hi. \u{1}V ok", "_:(en)'eItS _:'aI(ja)\n\n_:(en)'oU _:k'eI(ja)", "(en)ˈeɪtʃ ˈaɪ(ja)\n\n(en)ˈəʊ kˈeɪ(ja)", 84122, 0xa42a8f59ba1dea04, 0x80429b4621a5e3fe, X | IPA | WAV),
    ("ja", "a .\nb .\n\nc", "_:(en)'eI(ja)\n_:(en)b'i:(ja)\n_:(en)s'i:(ja)", "(en)ˈeɪ(ja)\n(en)bˈiː(ja)\n(en)sˈiː(ja)", 94812, 0xa56e2b05ba4c8014, 0x2191d66d99f06625, X | IPA),
    ("ja", "a ... b", "_:(en)'eI(ja)\n_:(en)b'i:(ja)", "(en)ˈeɪ(ja)\n(en)bˈiː(ja)", 52546, 0xa94aa6c4999bf581, 0x8993504186f4a0cf, ALL),
    ("ja", "Hi. Ok", "_:(en)'eItS _:'aI(ja)\n_:(en)'oU _:k'eI(ja)", "(en)ˈeɪtʃ ˈaɪ(ja)\n(en)ˈəʊ kˈeɪ(ja)", 84122, 0xa42a8f59ba1dea04, 0x5b018a9647ebec18, X | IPA | WAV),
    ("ja", "a. Ok", "_:(en)'eI(ja)\n_:(en)'oU _:k'eI(ja)", "(en)ˈeɪ(ja)\n(en)ˈəʊ kˈeɪ(ja)", 68554, 0xe30d490228c6a396, 0x9cb532ff204d09ab, X | IPA | WAV),
    ("ar", "? \u{1}V ! a", "(en)'eI(ar)", "(en)ˈeɪ(ar)", 24270, 0x4a4f4db052eb7ba2, 0x42ebeaa469b79b6b, ALL),
    ("ar", "a \u{1}V ! b", "(en)'eI(ar)\n(en)b'i:(ar)", "(en)ˈeɪ(ar)\n(en)bˈiː(ar)", 35736, 0x4117d3cb9f49c9ad, 0xb6af9516bf0f18b6, ALL),
    ("ar", "a ! \u{1}V b", "(en)'eI(ar)\n\n(en)b'i:(ar)", "(en)ˈeɪ(ar)\n\n(en)bˈiː(ar)", 52858, 0xa46529d8759f904d, 0xb45865a3870fe51b, ALL),
    ("ar", "a\u{1}V. Ok", "(en)'eI(ar)\n(en),oUk'eI(ar)", "(en)ˈeɪ(ar)\n(en)ˌəʊkˈeɪ(ar)", 42152, 0x869122f954b9df5a, 0x36af98b3b6c71514, X | IPA | WAV),
    ("ar", "Hi.\u{1}V ok", "(en)h'aI(ar)\n\n(en),oUk'eI(ar)", "(en)hˈaɪ(ar)\n\n(en)ˌəʊkˈeɪ(ar)", 61710, 0x745c17c803277df3, 0xc1d5f6228f2b6ec8, ALL),
    ("ar", "Hi. \u{1}V ok", "(en)h'aI(ar)\n\n(en),oUk'eI(ar)", "(en)hˈaɪ(ar)\n\n(en)ˌəʊkˈeɪ(ar)", 61710, 0x745c17c803277df3, 0x9934b8e9d229a09f, ALL),
    ("ar", "a .\nb .\n\nc", "(en)'eI(ar)\n(en)b'i:(ar)\n(en)s'i:(ar)", "(en)ˈeɪ(ar)\n(en)bˈiː(ar)\n(en)sˈiː(ar)", 87512, 0x9f85d6f46792f427, 0x195aba8bc25d0798, X | IPA),
    ("ar", "a ... b", "(en)'eI(ar)\n(en)b'i:(ar)", "(en)ˈeɪ(ar)\n(en)bˈiː(ar)", 47878, 0xca365e5c67fb69eb, 0x90f8219c52660093, ALL),
    ("ar", "Hi. Ok", "(en)h'aI(ar)\n(en),oUk'eI(ar)", "(en)hˈaɪ(ar)\n(en)ˌəʊkˈeɪ(ar)", 61710, 0x745c17c803277df3, 0xed734baa7148b493, ALL),
    ("ar", "a. Ok", "(en)'eI(ar)\n(en),oUk'eI(ar)", "(en)ˈeɪ(ar)\n(en)ˌəʊkˈeɪ(ar)", 57424, 0xc7746ad12520c0e1, 0xa4e5cc7b17c9c573, ALL),
    ("ko", "? \u{1}V ! a", "(en)'eI(ko)", "(en)ˈeɪ(ko)", 24270, 0x5ffdd505d0ea692f, 0x42ebeaa469b79b6b, ALL),
    ("ko", "a \u{1}V ! b", "(en)'eI(ko)\n(en)b'i:(ko)", "(en)ˈeɪ(ko)\n(en)bˈiː(ko)", 35834, 0x7e1d0faa3f156460, 0x33bfd28e0f9b9158, ALL),
    ("ko", "a ! \u{1}V b", "(en)'eI(ko)\n\n(en)b'i:(ko)", "(en)ˈeɪ(ko)\n\n(en)bˈiː(ko)", 53832, 0xa171b8d731f7d7db, 0xd901c55c684277e9, ALL),
    ("ko", "a\u{1}V. Ok", "(en)'eI(ko)\n(en),oUk'eI(ko)", "(en)ˈeɪ(ko)\n(en)ˌəʊkˈeɪ(ko)", 42222, 0x3d4a7f9f21262962, 0x283b3538e15fef9a, X | IPA | WAV),
    ("ko", "Hi.\u{1}V ok", "(en)h'aI(ko)\n\n(en),oUk'eI(ko)", "(en)hˈaɪ(ko)\n\n(en)ˌəʊkˈeɪ(ko)", 61698, 0xfd0fcacc815040be, 0xc6af74347d8723a8, ALL),
    ("ko", "Hi. \u{1}V ok", "(en)h'aI(ko)\n\n(en),oUk'eI(ko)", "(en)hˈaɪ(ko)\n\n(en)ˌəʊkˈeɪ(ko)", 61698, 0xfd0fcacc815040be, 0x36771605e879ae4f, ALL),
    ("ko", "a .\nb .\n\nc", "(en)'eI(ko)\n(en)b'i:(ko)\n(en)s'i:(ko)", "(en)ˈeɪ(ko)\n(en)bˈiː(ko)\n(en)sˈiː(ko)", 87532, 0x5b2851940c4377f9, 0x13454cc4b7af371c, X | IPA),
    ("ko", "a ... b", "(en)'eI(ko)\n(en)b'i:(ko)", "(en)ˈeɪ(ko)\n(en)bˈiː(ko)", 47700, 0xfa715e21fa699dbc, 0x97eed20cce7b68f9, ALL),
    ("ko", "Hi. Ok", "(en)h'aI(ko)\n(en),oUk'eI(ko)", "(en)hˈaɪ(ko)\n(en)ˌəʊkˈeɪ(ko)", 61698, 0xfd0fcacc815040be, 0x85cf760d4afed837, ALL),
    ("ko", "a. Ok", "(en)'eI(ko)\n(en),oUk'eI(ko)", "(en)ˈeɪ(ko)\n(en)ˌəʊkˈeɪ(ko)", 57418, 0xd00e18d20aa574e8, 0xe2e2f0feeaab0c08, ALL),
    ("hi", "? \u{1}V ! a", "(en)'eI(hi)", "(en)ˈeɪ(hi)", 22354, 0x18ab48b5fef4082f, 0x966f69ce8d39ba8a, ALL),
    ("hi", "a \u{1}V ! b", "(en)'eI(hi)\n(en)b'i:(hi)", "(en)ˈeɪ(hi)\n(en)bˈiː(hi)", 31366, 0xe91b0865af9db549, 0x83bf07b302468ba0, ALL),
    ("hi", "a ! \u{1}V b", "(en)'eI(hi)\n\n(en)b'i:(hi)", "(en)ˈeɪ(hi)\n\n(en)bˈiː(hi)", 48114, 0x299aa92348796e39, 0xdcd52954c911a2e7, ALL),
    ("hi", "a\u{1}V. Ok", "(en)'eI(hi)\n(en),oUk'eI(hi)", "(en)ˈeɪ(hi)\n(en)ˌəʊkˈeɪ(hi)", 37730, 0x160308b5cf7adcb8, 0x46e2ba8b492a2189, X | IPA | WAV),
    ("hi", "Hi.\u{1}V ok", "(en)h'aI(hi)\n\n(en),oUk'eI(hi)", "(en)hˈaɪ(hi)\n\n(en)ˌəʊkˈeɪ(hi)", 56138, 0x553d77d6a965dbf9, 0xe0fdbff81252a0e4, ALL),
    ("hi", "Hi. \u{1}V ok", "(en)h'aI(hi)\n\n(en),oUk'eI(hi)", "(en)hˈaɪ(hi)\n\n(en)ˌəʊkˈeɪ(hi)", 56138, 0x553d77d6a965dbf9, 0x8664cff82a72c4ff, ALL),
    ("hi", "a .\nb .\n\nc", "(en)'eI(hi)\n(en)b'i:(hi)\n(en)s'i:(hi)", "(en)ˈeɪ(hi)\n(en)bˈiː(hi)\n(en)sˈiː(hi)", 81174, 0x3ca7eaf05ffe67dd, 0x73908d31239c3760, X | IPA),
    ("hi", "a ... b", "(en)'eI(hi)\n(en)b'i:(hi)", "(en)ˈeɪ(hi)\n(en)bˈiː(hi)", 43034, 0xd4ecef3de90230d0, 0xc6d70973116e119e, ALL),
    ("hi", "Hi. Ok", "(en)h'aI(hi)\n(en),oUk'eI(hi)", "(en)hˈaɪ(hi)\n(en)ˌəʊkˈeɪ(hi)", 56138, 0x553d77d6a965dbf9, 0x464ff4eddd32091a, ALL),
    ("hi", "a. Ok", "(en)'eI(hi)\n(en),oUk'eI(hi)", "(en)ˈeɪ(hi)\n(en)ˌəʊkˈeɪ(hi)", 52954, 0xec0820c72b1a0058, 0x9e929c0ac176ca06, ALL),
    ("ru", "? \u{1}V ! a", "(en)'eI(ru)", "(en)ˈeɪ(ru)", 24270, 0x1acf5dc8c0135aee, 0x42ebeaa469b79b6b, ALL),
    ("ru", "a \u{1}V ! b", "(en)'eI(ru)\n(en)b'i:(ru)", "(en)ˈeɪ(ru)\n(en)bˈiː(ru)", 35736, 0x3a53f093955f7125, 0xb6af9516bf0f18b6, ALL),
    ("ru", "a ! \u{1}V b", "(en)'eI(ru)\n\n(en)b'i:(ru)", "(en)ˈeɪ(ru)\n\n(en)bˈiː(ru)", 52858, 0x14c0c8b9c9a65565, 0xb45865a3870fe51b, ALL),
    ("ru", "a\u{1}V. Ok", "(en)'eI(ru)\n(en),oUk'eI(ru)", "(en)ˈeɪ(ru)\n(en)ˌəʊkˈeɪ(ru)", 42152, 0x25ee6e4a5a7909ba, 0x36af98b3b6c71514, X | IPA | WAV),
    ("ru", "Hi.\u{1}V ok", "(en)h'aI(ru)\n\n(en),oUk'eI(ru)", "(en)hˈaɪ(ru)\n\n(en)ˌəʊkˈeɪ(ru)", 61710, 0xaf71813c7e31dc71, 0xc1d5f6228f2b6ec8, ALL),
    ("ru", "Hi. \u{1}V ok", "(en)h'aI(ru)\n\n(en),oUk'eI(ru)", "(en)hˈaɪ(ru)\n\n(en)ˌəʊkˈeɪ(ru)", 61710, 0xaf71813c7e31dc71, 0x9934b8e9d229a09f, ALL),
    ("ru", "a .\nb .\n\nc", "(en)'eI(ru)\n(en)b'i:(ru)\n(en)s'i:(ru)", "(en)ˈeɪ(ru)\n(en)bˈiː(ru)\n(en)sˈiː(ru)", 87512, 0x862730dc5f0727c9, 0x195aba8bc25d0798, X | IPA),
    ("ru", "a ... b", "(en)'eI(ru)\n(en)b'i:(ru)", "(en)ˈeɪ(ru)\n(en)bˈiː(ru)", 47878, 0x0093785db796bf5b, 0x90f8219c52660093, ALL),
    ("ru", "Hi. Ok", "(en)h'aI(ru)\n(en),oUk'eI(ru)", "(en)hˈaɪ(ru)\n(en)ˌəʊkˈeɪ(ru)", 61710, 0xaf71813c7e31dc71, 0xed734baa7148b493, ALL),
    ("ru", "a. Ok", "(en)'eI(ru)\n(en),oUk'eI(ru)", "(en)ˈeɪ(ru)\n(en)ˌəʊkˈeɪ(ru)", 57424, 0xc25deaf9e686f0dd, 0xa4e5cc7b17c9c573, ALL),
    ("he", "? \u{1}V ! a", "(en)'eI(he)", "(en)ˈeɪ(he)", 24270, 0x4a4f4db052eb7ba2, 0x42ebeaa469b79b6b, ALL),
    ("he", "a \u{1}V ! b", "(en)'eI(he)\n(en)b'i:(he)", "(en)ˈeɪ(he)\n(en)bˈiː(he)", 35736, 0x4117d3cb9f49c9ad, 0xb6af9516bf0f18b6, ALL),
    ("he", "a\u{1}V. Ok", "(en)'eI(he)\n(en),oUk'eI(he)", "(en)ˈeɪ(he)\n(en)ˌəʊkˈeɪ(he)", 42152, 0x869122f954b9df5a, 0x36af98b3b6c71514, X | IPA | WAV),
    ("he", "Hi.\u{1}V ok", "(en)h'aI(he)\n\n(en),oUk'eI(he)", "(en)hˈaɪ(he)\n\n(en)ˌəʊkˈeɪ(he)", 61710, 0x745c17c803277df3, 0xc1d5f6228f2b6ec8, ALL),
    ("he", "a .\nb .\n\nc", "(en)'eI(he)\n(en)b'i:(he)\n(en)s'i:(he)", "(en)ˈeɪ(he)\n(en)bˈiː(he)\n(en)sˈiː(he)", 87512, 0x9f85d6f46792f427, 0x195aba8bc25d0798, X | IPA),
    ("he", "a ... b", "(en)'eI(he)\n(en)b'i:(he)", "(en)ˈeɪ(he)\n(en)bˈiː(he)", 47878, 0xca365e5c67fb69eb, 0x90f8219c52660093, ALL),
    ("he", "a. Ok", "(en)'eI(he)\n(en),oUk'eI(he)", "(en)ˈeɪ(he)\n(en)ˌəʊkˈeɪ(he)", 57424, 0xc7746ad12520c0e1, 0xa4e5cc7b17c9c573, ALL),
    ("cmn", "? \u{1}V ! a", "(en)'eI55(cmn)_|", "(en)ˈeɪ5(cmn)", 20676, 0xd5c1f214a376cd44, 0xbbc4f66120cbfaaf, X | IPA),
    ("cmn", "a \u{1}V ! b", "(en)'eI55(cmn)_|\n(en)b'i:55(cmn)_|", "(en)ˈeɪ5(cmn)\n(en)bˈiː5(cmn)", 29700, 0x7abee727c6c00f31, 0x30a9fe61241a2a01, X | IPA),
    ("cmn", "a ! \u{1}V b", "(en)'eI55(cmn)_|\n\n(en)b'i:55(cmn)_|", "(en)ˈeɪ5(cmn)\n\n(en)bˈiː5(cmn)", 44648, 0xbc6b7c2edb501671, 0x30cfef2bf401b0cc, X | IPA),
    ("cmn", "a\u{1}V. Ok", "(en)'eI55(cmn)_|\n(en),oU55k'eI55(cmn)_|", "(en)ˈeɪ5(cmn)\n(en)ˌəʊəʊkˈeɪ5(cmn)", 36822, 0x6789f440d2737341, 0xddc9e7dddfd43e46, X | IPA),
    ("cmn", "Hi.\u{1}V ok", "(en)h'aI55(cmn)_|\n\n(en),oU55k'eI55(cmn)_|", "(en)hˈaɪ5(cmn)\n\n(en)ˌəʊəʊkˈeɪ5(cmn)", 52828, 0x8d182c2f311ae9eb, 0xb3bc338160b59494, X | IPA),
    ("cmn", "a .\nb .\n\nc", "(en)'eI55(cmn)_|\n(en)b'i:55(cmn)_|\n(en)s'i:55(cmn)_|", "(en)ˈeɪ5(cmn)\n(en)bˈiː5(cmn)\n(en)sˈiː5(cmn)", 75926, 0xa6948e742b002be9, 0x0d7394b7d5341128, X | IPA),
    ("cmn", "a ... b", "(en)'eI55(cmn)_|\n(en)b'i:55(cmn)_|", "(en)ˈeɪ5(cmn)\n(en)bˈiː5(cmn)", 39666, 0x6d91bc50a11ea391, 0xec82b9e35f5585e1, X | IPA),
    ("cmn", "a. Ok", "(en)'eI55(cmn)_|\n(en),oU55k'eI55(cmn)_|", "(en)ˈeɪ5(cmn)\n(en)ˌəʊəʊkˈeɪ5(cmn)", 50096, 0x9a6ee2cad0c8fce1, 0x0f5b32e902a3a870, X | IPA),
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
fn voice_change_marks_and_switch_ends_match_c() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = ROWS.iter().flat_map(differences).collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
