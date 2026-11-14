// A neutral-tone syllable counts as stressed when the tone pass looks for the
// last stressed syllable, which moves the clause's final pitch contour. The
// contour is only visible in the audio, so the expectations are the length and
// the FNV-1a hash of the 22050 Hz WAV from C espeak-ng master (f0fb11a8):
// `espeak-ng -w out.wav -v cmn <text>`.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3))
}

fn wav(text: &str, tag: &str) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!("cmn_neutral_{}_{tag}.wav", std::process::id()));
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-w", path.to_str().expect("utf-8 path"), "-v", "cmn", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .status()
        .expect("run cli");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav written");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// (text, WAV length in bytes, FNV-1a of the WAV)
const CASES: &[(&str, usize, u64)] = &[
    ("我们", 31338, 0x3f9a_8ecc_6434_664d),
    ("桌子", 35802, 0x2c19_e52c_1156_a368),
    ("妈妈", 27058, 0xa2d9_9a6a_72a1_c5f9),
    ("好吗", 32184, 0xf552_8398_9f77_80df),
];

#[test]
fn neutral_tone_final_syllable_gets_the_clause_final_contour() {
    for (i, (text, len, hash)) in CASES.iter().enumerate() {
        let bytes = wav(text, &i.to_string());
        assert_eq!((bytes.len(), fnv1a(&bytes)), (*len, *hash), "{text:?}");
    }
}
