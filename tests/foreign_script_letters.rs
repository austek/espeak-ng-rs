// tests/foreign_script_letters.rs
//
// A letter from a script the voice does not read is named by the language
// `alphabets[]` assigns to that script, with the alphabet's name first; a word
// the rules cannot read is spelled letter by letter.  Expected output is from
// C espeak-ng master.

use espeak_ng::Translator;

fn check(cases: &[(&str, &str, &str)]) {
    let mut failures = Vec::new();
    for &(lang, text, expected) in cases {
        let Ok(t) = Translator::new(lang, None) else { continue };
        let got = t.text_to_ipa(text).unwrap();
        if got != expected {
            failures.push(format!("{lang} {text:?}: got {got:?}, C says {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn cyrillic_is_named_in_english() {
    check(&[
        ("ku", "д", "(en)sɪɹˈɪlɪkdˈɛː(ku)"),
        ("de", "д", "kyːrˈɪlɪʃ(en)dˈɛː(de)"),
        (
            "fr",
            "Привет",
            "(en)sɪɹˈɪlɪkpˈɛː sɪɹˈɪlɪkˈɛr sɪɹˈɪlɪkˈɪː sɪɹˈɪlɪkvˈɛː sɪɹˈɪlɪkjˈɛː sɪɹˈɪlɪktˈɛː(fr)",
        ),
    ]);
}

#[test]
fn han_letters_say_the_script_and_the_voices_word_for_letter() {
    check(&[
        ("ku", "中", "(en)tʃˈaɪniːz(ku)tipa"),
        ("de", "中", "çiːnˈeːzɪʃbˈuːxʃtɑːbə"),
    ]);
}

#[test]
fn a_switched_letter_keeps_the_alphabet_name_in_the_voice_table() {
    check(&[("ku", "क", "(en)hˈɪndi(ku)kˈə"), ("de", "த", "(en)tˈamɪl(de)tˈɛ")]);
}
