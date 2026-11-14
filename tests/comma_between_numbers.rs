// A comma between two numbers is left to the number reading: it is not spoken
// as a stray mark by name. Values from upstream espeak-ng master (f0fb11a8),
// `espeak-ng -q --ipa -v <lang>` and `-q -x -v <lang>`.

fn cli(lang: &str, flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).replace('\n', "|")
}

/// (voice, text, `--ipa` stdout, `-x` stdout)
const CASES: &[(&str, &str, &str, &str)] = &[
    ("ab", "1,2", "ˈakə ˈɥba|", "'ak_@ 'Uwba|"),
    ("ab", "12,34", "ʒʷaˈɥa pʃbˈa|", "Zwa'Uwa pSjb'a|"),
    ("kn", "1,2", "ˈondu ˈeɹɐɖu|", "'ondu 'erad.u|"),
    ("kn", "12,34", "hˈɐnneɹˌɐɖu mˈovɐtʰnˈaːlku|", "h'anner,ad.u m'ovat#n'a:lku|"),
    ("ky", "1,2", "bˈir ekˈi|", "b'ir ek'i|"),
    ("ky", "12,34", "ˌonekˈi ot[ˈuzt[ˈoert[|", ",onek'i; ot'uzt'Ort|"),
    ("mr", "1,2", "ˈeːk dˈoːn|", "'e:k d'o:n|"),
    ("sd", "1,2", "hˈɪkʊ bˈa|", "h'IkU b'a|"),
    ("ur", "1,2", "ˈeːk dˈoː|", "'e:k d'o:|"),
    ("ur", "12,34", "bˈaːrah caʊŋtˈiːs|", "b'a:RaH caUNt'i:s|"),
];

#[test]
fn comma_between_numbers_is_not_spoken() {
    for (lang, text, ipa, x) in CASES {
        assert_eq!(&cli(lang, "--ipa", text), ipa, "{lang} {text:?}");
        assert_eq!(&cli(lang, "-x", text), x, "{lang} {text:?}");
    }
}
