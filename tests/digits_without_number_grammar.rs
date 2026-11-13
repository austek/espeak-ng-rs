// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v <lang>`.
// Languages with `langopts.numbers == 0` read digits one by one, in groups of
// three from the right once a run is longer than four.

fn cli(lang: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", "-x", "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

fn cases(lang: &str, rows: &[(&str, &str)]) {
    for (text, want) in rows {
        assert_eq!(&cli(lang, text), want, "{lang} {text:?}");
    }
}

#[test]
fn burmese_digits() {
    cases(
        "my",
        &[
            ("10", "t'i:tonj'a:_!"),
            ("25", "n'i:N'a:_!"),
            ("1 2 3", "t'i: n'i: t'ou:"),
            ("3.5", "t'ou:N'a:"),
            ("1234", "t'i:n'i:_!t'ou:l'i:_!"),
            ("1,234", "t'i: n'i:t'ou:_!l'i:"),
            ("1,000", "t'i: tonj'a:tonj'a:_!tonj'a:"),
            ("1 000", "t'i: tonj'a:tonj'a:_!tonj'a:"),
            ("12345", "t'i:n'i:_! t'ou:l'i:_!N'a:"),
            ("1234567", "t'i: n'i:t'ou:_!l'i: N'a:tSa:_!kon'i"),
            ("12345678", "t'i:n'i:_! t'ou:l'i:_!N'a: tS,a:kon'i_!S'i:"),
        ],
    );
}

#[test]
fn thai_digits_keep_their_tones_on_the_vowel() {
    cases(
        "th",
        &[
            ("1", "n'u5eng"),
            ("10", "nu5'e3ngsun_!"),
            ("1000", "n,u5e3ngsu3n_!s'u3nsun_!"),
            ("12345", "nu5'e5ngsong_! sa5ms'i3_!ha"),
            ("123456", "n,u5e5ngs'o3ng_!sam si3h'a3_!hok"),
            ("1234567", "n'u5eng so3ngs'a5m_!si ha3h'o3k_!chet"),
        ],
    );
}

#[test]
fn other_languages_without_a_number_grammar() {
    cases("mi", &[("10", "t,ahik'ORe_!_|"), ("1234567", "t'ahi_| R,uatOR'u_!fa:_| R,ima,ono_!f'itu_|")]);
    cases("qu", &[("10", "huktS?'usak_!"), ("1234567", "h'uk ,iskajk,imsa_!t'awa p,itSkas,ukta_!k'antSis")]);
    cases("uz", &[("10", "b'y*n'o:L_!"), ("1234567", "b'y* 'ykky'UtS_!t'o*t b'eSALt'y_!jet:'y")]);
    cases("piqd", &[("10", "wa?p'ar-_!"), ("1234567", "w'a? tSa?w'EdZ_!l'oS v'ar-dZ'av_!S'otS")]);
    cases("xex", &[("10", "n'Enu_!"), ("1234567", "nE dwotjE_!kwa peIljo_!Z'aI")]);
    cases("quc", &[("10", "j'un_!"), ("1234567", "j'un keb'oxib_!k'ajib jobw'aqib_!w'uqub")]);
}
