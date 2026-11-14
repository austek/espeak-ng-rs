// A letter word right after a number is translated with the number's last
// digit in its rule context, and is not dropped when the rules abandon it.
// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <lang>`.

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
fn letters_after_a_number_are_spelled_not_dropped() {
    let rows = [
        ("shn", "2abc", "s'O1N _:(en)'eI1 _:b'i:1 _:s'i:1(shn)"),
        ("my", "hak5ka", "_:(en)'eItS _:'eI _:k'eI(my) N'a: _:(en)k'eI _:'eI(my)"),
        ("am", "123abc", "m'@to hajas'ost (en),eIb,i:s'i:(am)"),
        ("am", "10km", "?'assyr (en)k,eI'Em(am)"),
        ("am", "5kg", "?'ammyst (en)k,eIdZ'i:(am)"),
        ("am", "2nd", "h'ul@t (en),End'i:(am)"),
        ("am", "3rd", "s'ost (en),A@d'i:(am)"),
        ("am", "1xy", "?'and (en),Eksw'aI(am)"),
        ("am", "1th", "?'and (en)t,i:;'eItS(am)"),
        ("am", "1sta", "?'and (en)st'A:(am)"),
    ];
    for (lang, text, want) in rows {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}

#[test]
fn an_english_ordinal_suffix_after_a_digit_is_silent_only_when_attached() {
    assert_eq!(cli("am", "1st"), "?'and");
    assert_eq!(cli("am", "21st"), "haja?'and");
    assert!(cli("am", "1 st").starts_with("?'and (en)s"));
}
