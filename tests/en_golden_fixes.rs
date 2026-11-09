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
fn start_of_word_rules_decide_whether_a_word_is_spelled() {
    check(&[
        ("st", "sˈənt"),
        ("BBC", "bˌiːbˌiːsˈiː"),
        ("brown", "bɹˈaʊn"),
        ("strength", "stɹˈɛŋθ"),
    ]);
}

#[test]
fn superscript_two_and_three_read_as_digits() {
    check(&[("²", "tˈuː"), ("³", "θɹˈiː"), ("x²", "ˈɛks tˈuː"), ("10³", "tˈɛn θɹˈiː")]);
}

#[test]
fn mismatched_ordinal_suffix_is_still_an_ordinal() {
    check(&[
        ("2st", "sˈɛkst"),
        ("3nd", "θˈɜːɹənd"),
        ("22st", "twˈɛnti sˈɛkst"),
        ("4rd", "fˈɔːd"),
        ("1st", "fˈɜːst"),
    ]);
}

#[test]
fn abbrev_flag_survives_a_suffix_stem_lookup() {
    check(&[("os", "ˌəʊˈɛs"), ("OS", "ˌəʊˈɛs")]);
}

#[test]
fn letters_of_other_alphabets_are_announced_by_alphabet_name() {
    check(&[
        ("ר", "hˈiːbɹuːrˈeʃ"),
        ("שלום", "hˈiːbɹuːʃin hˈiːbɹuːlˈamed hˈiːbɹuːvˈav hˈiːbɹuːmˈem"),
        ("ز", "ˈaɹəbɪk(ar)zˈaːj(en)"),
        ("ა", "dʒˈɔːdʒən(ka)ˈa(en)"),
        ("ক", "bɛŋɡˈɑːli(bn)kˈɔ(en)"),
        ("مرحبا", "(ar)mrħbˈaː(en)"),
    ]);
    check_phonemes(&[("ق", "_:'ar@bIk(ar)q'a.:f(en)")]);
}

#[test]
fn linking_r_before_a_vowel_is_changed_to_plain_r() {
    check_phonemes(&[
        ("meirath", "m'e@raT"),
        ("whereas", "w'e@raz"),
        ("purist", "pj'U@rIst"),
    ]);
}
