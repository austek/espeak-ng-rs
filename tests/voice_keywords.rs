// Voice-file keywords that change the audio.  Ratios are from upstream
// espeak-ng master (f0fb11a8): `espeak-ng -v <voice> -w out.wav "hello world"`
// gives 45394 bytes for `en`, 47250 with `stressAdd 0 0 -10 -10 0 0 10 40`
// and 59040 with `speed 80`.

use std::path::PathBuf;
use std::process::Command;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");

fn wav(args: &[&str], tag: &str) -> Vec<u8> {
    let out = std::env::temp_dir().join(format!("vk_{}_{tag}.wav", std::process::id()));
    let st = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(args)
        .arg("-w")
        .arg(&out)
        .arg("hello world")
        .env("ESPEAK_DATA_PATH", DATA)
        .status()
        .expect("run cli");
    assert!(st.success());
    let bytes = std::fs::read(&out).expect("wav written");
    let _ = std::fs::remove_file(&out);
    bytes
}

fn voice_file(tag: &str, body: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("vk_{}_{tag}.voice", std::process::id()));
    std::fs::write(&p, format!("language en\n{body}\n")).expect("write voice");
    p
}

fn ratio_to_default(body: &str, tag: &str) -> f64 {
    let base = wav(&["-v", "en"], &format!("{tag}b")).len() as f64;
    let p = voice_file(tag, body);
    let n = wav(&["--load", "-v", p.to_str().unwrap()], tag).len() as f64;
    let _ = std::fs::remove_file(&p);
    n / base
}

fn near(got: f64, want: f64) {
    assert!((got - want).abs() < 0.02, "ratio {got} vs C {want}");
}

#[test]
fn stress_add_in_a_voice_file_lengthens_stressed_syllables() {
    near(ratio_to_default("stressAdd 0 0 -10 -10 0 0 10 40", "sa"), 47250.0 / 45394.0);
}

#[test]
fn speed_keyword_scales_the_rate() {
    let p = voice_file("sp", "speed 80");
    let spoken = wav(&["--load", "-v", p.to_str().unwrap()], "sp").len();
    let _ = std::fs::remove_file(&p);
    // C: `speed 80` and `-s 140` both give 59040 bytes (175 * 80 / 100 = 140).
    assert_eq!(spoken, wav(&["-v", "en", "-s", "140"], "s140").len());
}

#[test]
fn stress_add_in_a_variant_changes_length_like_a_voice_file() {
    let base = wav(&["-v", "en"], "vb").len() as f64;
    let f2 = wav(&["-v", "en+f2"], "v2").len() as f64;
    near(f2 / base, 46.1 / 44.3);
}

#[test]
fn klatt_variant_changes_the_waveform() {
    let plain = wav(&["-v", "en"], "kp");
    let klatt = wav(&["-v", "en+klatt"], "kk");
    assert_ne!(plain, klatt);
    near(klatt.len() as f64 / plain.len() as f64, 1.0);
}
