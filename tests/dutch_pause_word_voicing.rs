// A `$pause` word such as `en` brings a pause with it. C's clause-wide voicing
// pass sees that pause between the words and leaves the preceding d voiced,
// where a plain neighbour devoices it. Expected strings are from C espeak-ng
// master.

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
fn consonant_before_a_pause_word_stays_voiced() {
    let cases = [
        ("een hond en twee katten", "_|@n h'Ond_:_: _|En tv#'e: k'At@n"),
        ("een bed en twee", "_|@n b'Ed_:_: _|En tv#'e:"),
        ("hond en twee", "h'Ont _|En tv#'e:"),
        // After a `$pause` word the next two words may not pause again.
        (
            "Dit brood of dat brood zijn goed.",
            "dIt br'o:d_:_: _|Of dAt br'o:t zEIn Q'ut",
        ),
        (
            "Ik hou van wijn en bier en brood.",
            "_|Ik h'VU vAn v#'EIn_:_: _|En b'ir _|En br'o:t",
        ),
    ];
    for (text, want) in cases {
        let Some(got) = mnemonics(text) else {
            eprintln!("[SKIP] no nl data");
            return;
        };
        assert_eq!(got, want, "{text}");
    }
}
