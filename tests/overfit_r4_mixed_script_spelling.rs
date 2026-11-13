// A spelled letter of a non-Arabic script next to a spelled Arabic letter keeps
// a single word break, whichever script comes first.  Values from upstream
// espeak-ng master (f0fb11a8), `espeak-ng -q <mode> -v <lang>`.

fn cli(mode: &str, lang: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", mode, "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

#[test]
fn mixed_script_letters_join_as_c_does() {
    let cases = [
        ("-x", "ku", "α ب", "(el)'alfa_: (en)'ar@bIk(ku)b'a:?"),
        ("--ipa", "ku", "α ب", "(el)ˈalfa (en)ˈaɹəbɪk(ku)bˈaːʔ"),
        ("-x", "ku", "ב ج", "_:(en)h'i:bru:_:(he)v'et_: (en)'ar@bIk(ku)dZ'i:m"),
        ("-x", "ku", "ب क", "_:(en)'ar@bIk(ku)b'a:?_: (en)h'Indi(ku)k'@"),
        ("--ipa", "ku", "ب Ա", "(en)ˈaɹəbɪk(ku)bˈaːʔ (en)ɑːmˈiːniən(ku)ˈaː"),
        ("-x", "ku", "ж ب", "_:(en)sI2r'IlIk_:Z'E:_: 'ar@bIk(ku)b'a:?"),
        ("-x", "sq", "α ب", "(el)'alfa_: (en)'ar@bIk(sq)b'a:?"),
    ];
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|&(mode, lang, text, want)| {
            let got = cli(mode, lang, text);
            (got != want).then(|| format!("{mode} {lang} {text:?}\n  got  {got}\n  want {want}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
