// A clock time keeps the zero of a zero-led minute before the seconds.  Values from
// upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <lang>`.

fn cli(lang: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn a_zero_minute_stays_before_the_seconds() {
    for (lang, text, want) in [
        ("cs", "10:05:30", "d'eset pj'et tR^'itset"),
        ("pl", "10:05:30", "dz;'ES;En^ts; p;'En^ts; tSydz;'ES;c;i"),
        ("de", "1:05:09", "_'aIns _|u:r f'ynf _|u:r n'Ul n'OYn"),
        ("de", "10:00:05", "ts'e:n _|u:r n'Ul _|u:r n'Ul f'ynf"),
        ("fr", "1:05:09", "'W~ 'Wr s'E~k 'Wr zero n'Wf"),
        ("ru", "1:05:09", "V#d;'in_ p;'At; n'ol d;'evI3t;"),
        ("it", "1:05:09", "'uno _e_ tS'inkwe _e_ dz'E*o n'Ove"),
        ("en", "12:00:01", "tw'Elv z'i@roU z'i@roU w'0n"),
    ] {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}
