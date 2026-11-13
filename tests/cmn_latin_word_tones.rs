// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v <lang>` and `-q -x -v <lang>`.

fn cli(lang: &str, flag: &str, text: &str) -> String {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", lang, "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    String::from_utf8_lossy(&out.stdout).trim().replace('\n', "|")
}

fn cases(lang: &str, flag: &str, rows: &[(&str, &str)]) {
    for (text, want) in rows {
        assert_eq!(&cli(lang, flag, text), want, "{lang} {flag} {text:?}");
    }
}

#[test]
fn latin_words_take_the_sandhi_tone_in_ipa() {
    cases(
        "cmn",
        "--ipa",
        &[
            ("ABC公司", "(en)ˌeɪ5bˌiː5sˈiː5(cmn) kˈonɡ5 sˈi̪5"),
            ("CEO说话", "(en)sˌiː5ˌiː5ˈəʊəʊ(cmn) s.wˈo5 χwˈɑ5"),
        ],
    );
    cases("cmn-latn-pinyin", "--ipa", &[("ni hao", "(en)nˈaɪ5 hˈaʊ5(cmn)"), ("ABC", "(en)ˌeɪ5bˌiː5sˈiː5(cmn)")]);
}

#[test]
fn latin_words_take_the_sandhi_tone_in_phoneme_text() {
    cases(
        "cmn",
        "-x",
        &[
            ("ABC公司", "(en),eI55b,i:55s'i:55(cmn)_| k'ong55_| s'i[55_|"),
            ("公司ABC", "k'ong55_| s'i[55_| (en)eI22bi:11si:11(cmn)_|"),
            ("你 a 好", "n'i21_| (en)eI44(cmn)_| X'Au214_|"),
            ("中文 English 中文", "ts.'ong55_| w'u@35n_| (en)I33NglI11S(cmn)_| ts.'ong55_| w'u@35n_|"),
        ],
    );
}

#[test]
fn the_pause_after_a_latin_word_follows_the_closing_tag() {
    cases(
        "cmn",
        "-x",
        &[("hello 世界", "(en)h@55l'oU55(cmn)_| s.'i.51_| tS;'iE51_|"), ("他说：hello，你好", "th'A55_| s.w'o55_||(en)h@55l'oU55(cmn)_||ni35X'Au214_|")],
    );
}
