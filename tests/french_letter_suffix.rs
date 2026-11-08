// tests/french_letter_suffix.rs
//
// The fr dictionary has no `_#<suffix>` ordinal markers, so C reads a number
// followed by letters as the cardinal, then the letters as a word ("2e" is
// "deux" + "e"), not as "deuxième".  Expected output is from C espeak-ng master.

use espeak_ng::Translator;

fn ipa(text: &str) -> Option<String> {
    let Ok(t) = Translator::new("fr", None) else {
        eprintln!("[SKIP] no fr data");
        return None;
    };
    Some(t.text_to_ipa(text).unwrap())
}

#[test]
fn letter_suffix_reads_number_then_letters() {
    for (text, expected) in [
        ("1re", "ˈœ̃ ʁə-"),
        ("2e", "dˈø ˈə"),
        ("3e", "tʁwˈa ˈə"),
        ("2nd", "dˈøz ˌɛndˈe"),
        ("3ème", "tʁwˈaz jˈɛm"),
        ("21e", "vɛ̃teˈœ̃ ˈə"),
        ("2es", "dˈøz ˈɛ"),
        ("5e", "sˈɛ̃k ˈə"),
        ("10e", "dˈis ˈə"),
        ("100e", "sˈɑ̃ ˈə"),
        ("1000e", "mˈil ˈə"),
        ("1e", "ˈœ̃ ˈə"),
        ("80e", "katʁəvˈɛ̃ ˈə"),
        ("81e", "katʁəvɛ̃ˈœ̃ ˈə"),
        ("le 2e jour", "lə- dˈø ˈə ʒˈuʁ"),
    ] {
        if let Some(got) = ipa(text) {
            assert_eq!(got, expected, "fr {text}");
        }
    }
}

#[test]
fn dictionary_ordinals_stay_ordinal() {
    for (text, expected) in [
        ("1er", "pʁəmjˈe"),
        ("1ers", "pʁəmjˈe"),
        ("1ère", "pʁəmjˈɛʁ"),
        ("1ère année", "pʁəmjˈɛʁ anˈe"),
        ("1ER", "pʁəmjˈe"),
        ("le 1er mai", "lə- pʁəmjˈe mˈɛ"),
        ("1ères", "pʁəmjˈɛʁ"),
        ("1èrement", "pʁəmjɛʁmˈɑ̃"),
        ("11er", "pʁəmjˈe"),
        ("21er", "pʁəmjˈe"),
        ("31er", "tʁɑ̃tpʁəmjˈe"),
        ("71er", "swasɑ̃tpʁəmjˈe"),
        ("101er", "sɑ̃ pʁəmjˈe"),
        ("01er", "zeʁo pʁəmjˈe"),
        ("21ème", "vɛ̃teynjˈɛm"),
        ("81ème", "yitɑ̃teynjˈɛm"),
        ("121ème", "sɑ̃ vɛ̃teynjˈɛm"),
        ("2021ème", "dø mil vɛ̃teynjˈɛm"),
        ("21ème étage", "vɛ̃teynjˈɛm etˈaʒ"),
    ] {
        if let Some(got) = ipa(text) {
            assert_eq!(got, expected, "fr {text}");
        }
    }
}
