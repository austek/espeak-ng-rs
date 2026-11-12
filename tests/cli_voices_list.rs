// `--voices` and `--voices=<lang>` print C's table byte for byte: columns, ordering, scoring and which voices are listed.

use std::process::Command;

const HEADER: &str =
    "Pty Language       Age/Gender VoiceName          File                 Other Languages\n";

fn voices(arg: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .env("ESPEAK_DATA_PATH", env!("CARGO_MANIFEST_DIR"))
        .arg(arg)
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn table(rows: &[&str]) -> String {
    rows.iter().fold(HEADER.to_string(), |acc, r| format!("{acc}{r}\n"))
}

#[test]
fn all_voices() {
    assert_eq!(voices("--voices"), include_str!("cli_voices/all.txt"));
}

#[test]
fn unknown_language_lists_only_the_header() {
    assert_eq!(voices("--voices=zz"), HEADER);
}

#[test]
fn language_with_mbrola_voices_orders_by_score_then_name() {
    let want = table(&[
        " 5  pt              --/M      Portuguese_(Portugal) roa/pt               (pt-pt 5)",
        " 5  pt-br           --/M      Portuguese_(Brazil) roa/pt-BR            (pt 6)",
        " 7  pt-pt           --/F      portugal-mbrola-1  mb/mb-pt1            (pt 7)",
        " 6  pt-br           --/M      brazil-mbrola-1    mb/mb-br1            (pt 8)",
        " 6  pt-br           --/F      brazil-mbrola-2    mb/mb-br2            (pt 8)",
        " 6  pt-br           --/M      brazil-mbrola-3    mb/mb-br3            (pt 8)",
        " 6  pt-br           --/F      brazil-mbrola-4    mb/mb-br4            (pt 8)",
    ]);
    assert_eq!(voices("--voices=pt"), want);
}

#[test]
fn other_languages_follow_the_file_column() {
    let want = table(&[
        " 5  cmn             --/M      Chinese_(Mandarin,_latin_as_English) sit/cmn              (zh-cmn 5)(zh 5)",
        " 5  cmn-latn-pinyin --/M      Chinese_(Mandarin,_latin_as_Pinyin) sit/cmn-Latn-pinyin  (zh-cmn 5)(zh 5)",
    ]);
    assert_eq!(voices("--voices=cmn"), want);
}

#[test]
fn name_keeps_the_space_before_a_trailing_comment() {
    let all = voices("--voices");
    assert!(all.contains("Cherokee_          iro/chr"), "{all}");
}
