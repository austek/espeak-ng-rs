// A colon between digit strings is a word of its own that the language's rules read with
// the digits around it as context (en `D_) : (_DD_` silent, de `_L05_) : (_L06D_` "Uhr",
// it/lij "e", fr "heures", otherwise the bare `:` rule or nothing).  Values from upstream
// espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <lang>`.

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
fn colon_reading_follows_the_rule_context() {
    for (lang, text, want) in [
        ("lij", "1:05:09", "'yN _e_ S'inkwe _e_ z,e:Ru n'Y:ve"),
        ("de", "10::30", "ts'e:n _|u:r dr'aIsIC"),
        ("fr", "10:30", "d'iz 'Wr tr'A~t"),
        ("de", "25:30", "f'ynf _|Unttsv'antsIC dr'aIsIC"),
        ("lt", "25:30", "dv;'Id;eS;imtp;enk'I t@-r;'Isd;eS;imt"),
        ("lt", "10:30.5", "d;'eS;imt d;v;'ItaS;k;is t@-r;'Isd;eS;imt t'aSkas p;enk'I"),
        ("en", "3::2", "Tr'i: k'oUl@n t'u:"),
        ("lt", "3::2", "t@-r'i:s d;v;'ItaS;k;is d'u"),
        ("lv", "3::2", "tR'i:s k'o:ls d'ivi"),
        ("en", "1.5:30", "w'0n pOInt f'aIv T'3:ti"),
    ] {
        assert_eq!(cli(lang, text), want, "{lang} {text:?}");
    }
}
