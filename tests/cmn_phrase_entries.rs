// Mandarin multi-character list entries such as `(你 好) ni3hao3` match across
// the per-character words, and a text-mode entry keeps the words it consumed.
// Expected mnemonics are from C espeak-ng master.

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
fn phrase_entries_join_their_characters() {
    let cases = [
        ("你好", "ni35X'Au214_|"),
        ("小老虎", "S;j'Au35_| lAu35X'u214_|"),
        ("我很好", "w'o35_| X@35nX'Au214_|"),
        ("你也好", "n'i35_| jiE35X'Au214_|"),
        ("我也有", "w'o35_| jiE35j'iou214_|"),
        ("展览馆", "ts.a35nl'a35n_| kw'a214n_|"),
        ("处理", "ts.hu35l'i214_|"),
        ("了解", "ljAu35tS;'iE214_|"),
        ("好久不见", "X,Au35tS;iou21pu44tS;'iE51n_|"),
        ("我想买雨伞", "w'o35_| S;'iA35N_| m'ai35_| jy35s'a214n_|"),
    ];
    for (text, want) in cases {
        let Some(got) = mnemonics("cmn", text) else {
            eprintln!("[SKIP] no data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}

#[test]
fn unlisted_pairs_stay_separate_words() {
    let cases = [
        ("我买", "w'o35_| m'ai214_|"),
        ("买米", "m'ai35_| m'i214_|"),
        ("每晚", "m'ei35_| w'A214n_|"),
        ("你有几本", "n'i35_| j'iou35_| tS;'i35_| p'@214n_|"),
    ];
    for (text, want) in cases {
        let Some(got) = mnemonics("cmn", text) else {
            eprintln!("[SKIP] no data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
