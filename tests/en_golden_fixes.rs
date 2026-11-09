// Values from upstream espeak-ng master (f0fb11a8), `espeak-ng -q --ipa -v en`.

use espeak_ng::Translator;

fn check(cases: &[(&str, &str)]) {
    let Ok(t) = Translator::new("en", Some(std::path::Path::new("espeak-ng-data"))) else {
        eprintln!("[SKIP] no en data");
        return;
    };
    for (word, want) in cases {
        let got = t.text_to_ipa(word).unwrap_or_default();
        assert_eq!(&got, want, "word {word:?}");
    }
}

#[test]
fn magic_e_rule_silences_the_next_e() {
    check(&[
        ("dofeet", "dˈəʊfɪt"),
        ("fubypeep", "fˈʌbaɪpˌɛp"),
        ("dopeep", "dˈəʊpɛp"),
        ("deetroneey", "dˈiːtɹəʊni"),
        ("poseag", "pˈəʊsaɡ"),
    ]);
}

fn check_phonemes(cases: &[(&str, &str)]) {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    for (text, want) in cases {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
            .args(["-q", "-x", "-v", "en", "--", text])
            .env("ESPEAK_DATA_PATH", data)
            .output()
            .expect("run cli");
        let got = String::from_utf8_lossy(&out.stdout).trim().replace('\n', " ");
        assert_eq!(&got, want, "text {text:?}");
    }
}

#[test]
fn linking_r_before_a_vowel_is_changed_to_plain_r() {
    check_phonemes(&[
        ("meirath", "m'e@raT"),
        ("whereas", "w'e@raz"),
        ("purist", "pj'U@rIst"),
    ]);
}
