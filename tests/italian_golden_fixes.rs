// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q -x -v it`.

fn check_phonemes(cases: &[(&str, &str)]) {
    check_with("-x", cases);
}

fn check_ipa(cases: &[(&str, &str)]) {
    check_with("--ipa", cases);
}

fn check_with(flag: &str, cases: &[(&str, &str)]) {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    for (text, want) in cases {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
            .args(["-q", flag, "-v", "it", "--", text])
            .env("ESPEAK_DATA_PATH", data)
            .output()
            .expect("run cli");
        let got = String::from_utf8_lossy(&out.stdout).trim().replace('\n', " ");
        assert_eq!(&got, want, "text {text:?}");
    }
}

#[test]
fn lengthened_vowel_still_glides_into_a_vowel() {
    check_phonemes(&[
        ("spie", "sp'i:;e"),
        ("siano", "s'i:;ano"),
        ("stiano", "st'i:;ano"),
        ("lucia", "lUtS'i:;a"),
    ]);
    check_ipa(&[("spie", "spˈiːe"), ("siano", "sˈiːano"), ("lucia", "lʊtʃˈiːa")]);
}

#[test]
fn word_pause_replaces_the_glide_at_a_word_end() {
    check_phonemes(&[
        ("adsl", "a dI_| esse_| 'Elle"),
        ("ibm", "i bI_| 'Emme"),
    ]);
}

#[test]
fn cyrillic_letter_handed_to_russian_is_announced() {
    check_phonemes(&[
        ("м", "_:tSIR'illIko_:(ru)'Em(it)"),
        ("ш", "_:tSIR'illIko_:(ru)S'A(it)"),
        ("x м", "'iks _:tSIR'illIko_:(ru)'Em(it)"),
    ]);
    check_ipa(&[("ц", "tʃɪrˈillɪko(ru)tsˈɛ(it)"), ("ж", "tʃɪrˈillɪko(ru)ʒˈɛ(it)")]);
}

#[test]
fn syllabic_marker_after_a_word_break_takes_no_syllable() {
    check_phonemes(&[("ъ", "jer dUR'a"), ("й", "I b@-*ev'e")]);
    check_ipa(&[("ъ", "jer dʊrˈa"), ("й", "ɪ brevˈe")]);
}

#[test]
fn end_stress_is_followed_by_the_alternate_vowel() {
    check_phonemes(&[("del", "d'el"), ("dal", "d'al")]);
}

#[test]
fn word_entry_flags_win_over_the_stem_flags() {
    check_phonemes(&[
        ("privilegia", "p@-*ivil'edZa"),
        ("ciascune", "tSask'une"),
        ("agite", "adZ'ite"),
        ("sedilo", "s'Edilo"),
    ]);
}
