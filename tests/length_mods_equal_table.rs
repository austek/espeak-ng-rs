// `SetLengthMods(tr, 3)` selects C's `length_mods_equal`, whose `t` row holds 100 (not 110) in the
// `n` and `N` columns.  Hungarian vowels before a nasal and a stop take that cell.  A lone voiced
// stop (`ggy`) also reads `amp` of the closing pause, which `MakePhonemeList` leaves as the
// previous clause left that slot of `phoneme_list` (0 in a fresh process).  Expected
// values come from `espeak-ng -v hu -w out.wav <text>` of espeak-ng f0fb11a8: the sample count
// ((file bytes - 44) / 2) and the FNV-1a 64 hash of the PCM bytes.

use std::process::Command;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

fn render(lang: &str, text: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("length_mods_equal_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{lang}_{:x}.wav", text.bytes().map(usize::from).sum::<usize>()));
    let status = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-v", lang, "-w"])
        .arg(&path)
        .arg(text)
        .status()
        .expect("run");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("wav");
    let _ = std::fs::remove_file(&path);
    bytes[44..].to_vec()
}

#[test]
fn hungarian_words_match_c_sample_for_sample() {
    if !std::path::Path::new("espeak-ng-data/hu_dict").exists() {
        eprintln!("[SKIP] no data");
        return;
    }
    let cases: [(&str, usize, u64); 11] = [
        ("lementeni", 19262, 0x1795be88af230de9),
        ("döntései", 20359, 0x3d46e47787943b14),
        ("forintot", 19586, 0x5de6d311441158a8),
        ("álláspontja", 25251, 0x32d45654b6e70ffc),
        ("bementek", 18352, 0x95faba2efea45006),
        ("ggy", 8731, 0x9b60b7a3b8f2c6fd),
        ("jelentene", 19749, 0xc3467da84a577f09),
        ("kijelenteni", 22605, 0x647ebc2216cf69c8),
        ("megoldanunk", 22066, 0xbaf466f0b58e5ed0),
        ("ggy. ggy", 11203, 0xc4abe21c4956b0b3),
        ("bementek. ggy. lementeni, ggy", 37942, 0xf4d916ffb610a418),
    ];
    let bad: Vec<String> = cases
        .iter()
        .filter_map(|&(text, n, hash)| {
            let pcm = render("hu", text);
            (pcm.len() / 2 != n || fnv1a(&pcm) != hash).then(|| format!("{text:?}: {} samples (C {n})", pcm.len() / 2))
        })
        .collect();
    assert!(bad.is_empty(), "differs from C:\n{}", bad.join("\n"));
}
