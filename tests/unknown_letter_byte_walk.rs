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
fn a_letter_of_a_two_letter_group_that_matches_no_rule_is_passed_over() {
    cases("ky", "-x", &[("кгб", "b"), ("жкт", "dZt")]);
    cases("ky", "--ipa", &[("кгб", "b"), ("жкт", "dZt[")]);
}

#[test]
fn a_two_letter_group_ending_inside_a_letter_resumes_on_its_last_byte() {
    cases("my", "-x", &[("ႀက", "k")]);
    cases("my", "--ipa", &[("ႀက", "k")]);
}

#[test]
fn myanmar_reads_a_shan_word_letter_by_letter() {
    cases(
        "my",
        "-x",
        &[("သွၼ်", "D w'a _:(en)mj'A:nmA@(my)l'et@_|t'i:_|tonj'a:_|kon'i_|s'i:_: _|")],
    );
    cases(
        "my",
        "--ipa",
        &[("သွၼ်", "ð wˈa (en)mjˈɑːnmɑː(my)lˈetətˈiːtonjˈaːkonˈisˈiː")],
    );
}

#[test]
fn shan_spells_the_unknown_letter_after_its_known_letters() {
    cases(
        "shn",
        "-x",
        &[("သွၼ်", "s _:(en)mj'A:1nmA@1(shn)l'e1t@1_|n'E3N_|s'o1n_|s'a:1m_|d'i:1_: n")],
    );
    cases(
        "shn",
        "--ipa",
        &[("သွၼ်", "s (en)mjˈɑː1nmɑːɑː(shn)lˈe1təənˈɛɛŋsˈo1nsˈaːaːmdˈiː1 n")],
    );
}
