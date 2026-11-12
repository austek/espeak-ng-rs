// tests/synth_parity_langs.rs
//
// Byte-for-byte WAV parity with C espeak-ng master (f0fb11a8) for languages
// whose per-language synthesis settings come from `SelectTranslator`.  Each
// expected value is the FNV-1a 64 hash of the PCM samples (WAV header
// stripped) the C binary writes, plus its sample count.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(args: &[&str], text: &str) -> Vec<u8> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let out: PathBuf = std::env::temp_dir().join(format!("espeak-rs-spl-{}-{n}.wav", std::process::id()));
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(args)
        .arg("-w")
        .arg(&out)
        .arg(text)
        .status()
        .expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&out).expect("read wav");
    let _ = std::fs::remove_file(&out);
    bytes[44..].to_vec()
}

/// (args, text, C sample count, C PCM hash)
const CASES: &[(&[&str], &str, usize, u64)] = &[
    (&["-v", "it"], "Buongiorno a tutti, come state oggi?", 51177, 0xff45844b1c42186d),
    (&["-v", "it"], "La casa è molto grande.", 34412, 0x773cbe869a89e366),
    (&["-v", "it"], "Domani andiamo al mare con gli amici.", 47402, 0x68297214dc580b0a),
    (&["-v", "pt"], "Bom dia a todos, como vão vocês?", 51680, 0x3c52efe57c38792c),
    (&["-v", "pt"], "A casa é muito grande.", 36214, 0xfc924af86e08ad61),
    (&["-v", "pt"], "Amanhã vamos à praia com os amigos.", 51428, 0xe4b7f2d3b4f2aecd),
    (&["-v", "nl"], "Het huis is erg groot.", 37329, 0xc307e8960f9fa577),
    (&["-v", "nl"], "Morgen gaan we naar zee met vrienden.", 51896, 0xff10b2459c615574),
    (&["-v", "pl"], "Dzień dobry wszystkim, jak się macie?", 58338, 0x0f723e26b08b0668),
    (&["-v", "pl"], "Ten dom jest bardzo duży.", 41259, 0xcdc120ff52a61ccb),
    (&["-v", "pl"], "Jutro jedziemy nad morze z przyjaciółmi.", 54652, 0x3a18c41de3a5c49c),
    (&["-v", "cs"], "Dobré ráno všem, jak se máte?", 48929, 0x61ee2663927d5153),
    (&["-v", "cs"], "Ten dům je velmi velký.", 36870, 0x8c7004fe77c813ac),
    (&["-v", "cs"], "Zítra jedeme k moři s přáteli.", 47088, 0x68828d6cb4092fd7),
    (&["-v", "sv"], "God morgon allihopa, hur mår ni?", 30758, 0x21fe431d9e1a4068),
    (&["-v", "sv"], "Huset är mycket stort.", 20469, 0x363a2979eeac0f2f),
    (&["-v", "sv"], "I morgon åker vi till havet med vänner.", 33959, 0xafe6a988c94a5da6),
    (&["-v", "el"], "Καλημέρα σε όλους, τι κάνετε;", 43671, 0xd84d28b391ff9449),
    (&["-v", "el"], "Το σπίτι είναι πολύ μεγάλο.", 35359, 0x87b7704229010d10),
    (&["-v", "el"], "Αύριο πάμε στη θάλασσα με φίλους.", 49127, 0xb2ddcc289e476eb3),
    (&["-v", "hi"], "यह घर बहुत बड़ा है।", 34319, 0xb172459476ebbb43),
    (&["-v", "tr"], "Herkese günaydın, nasılsınız?", 53556, 0x6d655793e52932b8),
    (&["-v", "tr"], "Bu ev çok büyük.", 29325, 0x7af5a47720d078b2),
    (&["-v", "tr"], "Yarın arkadaşlarla denize gidiyoruz.", 59181, 0x52f9d230c1a35582),
    (&["-v", "ro"], "Bună dimineața tuturor, ce mai faceți?", 58664, 0x76dc02f87a7edd06),
    (&["-v", "ro"], "Casa este foarte mare.", 37501, 0x1b9c55202473d837),
    (&["-v", "ro"], "Mâine mergem la mare cu prietenii.", 47867, 0xa8966d640c862f8b),
    (&["-v", "da"], "Huset er meget stort.", 30865, 0x00d06c096b2566d2),
    (&["-v", "am"], "ቤቱ በጣም ትልቅ ነው።", 36719, 0x7c74b4fcf762f1e6),
    (&["-v", "am"], "ነገ ከጓደኞች ጋር ወደ ባሕር እንሄዳለን።", 68340, 0x882262a1f15ae6a2),
    (&["-v", "eo"], "Bonan matenon al ĉiuj, kiel vi fartas?", 57929, 0xfc4399652f0fe0b7),
    (&["-v", "eo"], "La domo estas tre granda.", 37952, 0xe72d08dc89daccf3),
    (&["-v", "eo"], "Morgaŭ ni iros al la maro kun amikoj.", 52947, 0xf30e7c8dd531ad1d),
];

#[test]
fn language_settings_match_c_byte_for_byte() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let bad: Vec<String> = CASES
        .iter()
        .filter_map(|&(args, text, n, hash)| {
            let pcm = render(args, text);
            (pcm.len() / 2 != n || fnv1a(&pcm) != hash)
                .then(|| format!("{args:?} {text:?}: {} samples (C {n})", pcm.len() / 2))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
