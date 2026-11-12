// `-v` accepts a voice file path relative to the voices directory and loads that
// file's own settings (its dictionary and rules), as C does. `gmq/nb` names the
// language `nb` but uses the `no` dictionary; `bat/ltg` uses Latvian data with
// the Latgalian rule set.

use std::process::Command;

const TEXT: &str = "bonjour šodien cilvēki kjære takk";
const NORWEGIAN: &str = "b'Onju:,u-:R s'u:di:;@n s'Ilve:x,i: x'a:Ra# t'Ak:\n";
const LATGALIAN: &str = "b'onjouR S'uodien ts'illvE:ki kj'aRe t'akk\n";
const LATVIAN: &str = "b'uonjouR S'uodien ts'illvE:ki kj'aRe t'akk\n";

fn x(voice: &str, text: &str) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .env("ESPEAK_DATA_PATH", env!("CARGO_MANIFEST_DIR"))
        .args(["-q", "-x", "-v", voice, text])
        .output()
        .expect("spawn");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn speaks(voice: &str, phonemes: &str) {
    assert_eq!(x(voice, TEXT), (Some(0), phonemes.into(), String::new()), "-v {voice}");
}

#[test]
fn voice_file_with_its_own_dictionary_name() {
    speaks("gmq/nb", NORWEGIAN);
    speaks("gmq/da", "b?VNZ'ur s'?oD?i@-n s'?ilv?ek,?i kj'E?V_! t'?Ak\n");
}

#[test]
fn voice_file_rules_differ_from_the_dictionary_language() {
    speaks("bat/ltg", LATGALIAN);
    speaks("lv", LATVIAN);
}

#[test]
fn variant_suffix_applies_to_a_voice_file_path() {
    speaks("gmq/nb+f2", NORWEGIAN);
    speaks("bat/ltg+m3", LATGALIAN);
}

#[test]
fn simple_voice_file_paths() {
    assert_eq!(x("art/eo", "saluton"), (Some(0), "sal'uton\n".into(), String::new()));
    assert_eq!(x("aav/vi", "xin chao"), (Some(0), "s'i1n_| tS'a:7w_|\n".into(), String::new()));
}
