// A multi-word list entry such as `(de een)` pronounces its words together and
// the words after the first are left out. Expected mnemonics are from C
// espeak-ng master.

use std::path::Path;
use std::process::Command;

fn mnemonics(text: &str) -> Option<String> {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    if !Path::new(data).join("nl_dict").exists() {
        return None;
    }
    let out = Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", "nl", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().replace('\n', " "))
}

#[test]
fn phrase_entries_are_stressed_as_one_word() {
    let cases = [
        ("de een", "d@ 'e:n"),
        ("het een", "h@t 'e:n"),
        ("een of", "_!'e:n Of"),
        ("een van", "_!'e:n vAn"),
        ("niet meer", "_n'it m,e:r"),
        ("wil je een of twee", "v#Il j@ _!'e:n Of tv#'e:"),
        ("hij heeft niet meer", "hEI he:ft _n'it m,e:r"),
    ];
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no nl data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
