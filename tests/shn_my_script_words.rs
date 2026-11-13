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

const TAIL: &str = "_:(en)mj'A:1nmA@1(shn)l'e1t@1_|n'E3N_|s'o1n_|s'a:1m_|p'E2t_:";

#[test]
fn shan_tone_marks_belong_to_a_word_of_their_own() {
    cases("shn", "-x", &[("ၵ်ႈ", "k"), ("ၵိၵ်ႈ", "k'i5k"), ("ၵႂ်", "k")]);
}

#[test]
fn an_unknown_letter_is_named_with_the_default_tone_on_every_vowel() {
    let want = format!("k {TAIL}");
    cases("shn", "-x", &[("ၵ်း", want.as_str())]);
    cases(
        "shn",
        "--ipa",
        &[("ၵ်း", "k (en)mjˈɑː1nmɑːɑː(shn)lˈe1təənˈɛɛŋsˈo1nsˈaːaːmpˈɛɛt")],
    );
}

#[test]
fn a_word_with_an_unknown_letter_inside_is_spelled() {
    cases(
        "shn",
        "-x",
        &[
            ("သွၼ်", "s _:(en)mj'A:1nmA@1(shn)l'e1t@1_|n'E3N_|s'o1n_|s'a:1m_|d'i:1_: n"),
            ("abc", "_:(en)'eI1 _:b'i:1 _:s'i:1(shn)"),
        ],
    );
    cases("my", "-x", &[("hello", "_:(en)'eItS _:'i: _:'El _:'El _:'oU(my)")]);
}

#[test]
fn hakka_tones_print_as_digits_in_ipa() {
    cases("hak", "--ipa", &[("a1 e2 o3", "ˈa1 ˈe2 ˈoɜ")]);
}
