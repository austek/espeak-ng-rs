// In pinyin, `u` after j, q, x or y is the rounded front vowel, also before a
// final `n` that carries a tone digit (`xun4`, `quan2`): a post-context digit
// matches when no digit follows. Expected mnemonics are from C espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(lang: &str, text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("cmn_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', "/"))
}

#[test]
fn u_before_n_after_palatal_initial() {
    let cases = [
        ("cmn", "xun4", "S;'y@51n_|"),
        ("cmn", "quan2", "tS;h'y&35n_|"),
        ("cmn", "yun2", "j'y@35n_|"),
        ("cmn", "jun1", "tS;'y@55n_|"),
        ("cmn", "xuan2", "S;'y&35n_|"),
        ("cmn", "lun2", "l'u@35n_|"),
        ("cmn", "讯", "S;'y@51n_|"),
        ("cmn", "版权", "p'a21n_| tS;h'y&35n_|"),
        ("cmn", "拳击手套", "tS;h'y&35n_| tS;'i55_| s.'ou21_| th'Au51_|"),
        ("cmn-latn-pinyin", "xun4", "S;'y@51n_|"),
    ];
    for (lang, text, want) in cases {
        let Some(got) = mnemonics(lang, text) else {
            eprintln!("[SKIP] no data");
            return;
        };
        assert_eq!(got, want, "{lang} {text}");
    }
}
