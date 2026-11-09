// A point between digits that is not the language's decimal separator, and a
// continued decimal point (`1.2.3`).  Expected strings are C espeak-ng master.
use espeak_ng::Translator;

fn check(cases: &[(&str, &str, &str)]) {
    for (lang, text, expected) in cases {
        let Ok(t) = Translator::new_default(lang) else {
            eprintln!("[SKIP] no {lang} data");
            continue;
        };
        assert_eq!(&t.text_to_ipa(text).unwrap(), expected, "{lang} {text}");
    }
}

#[test]
fn point_is_the_unstressed_entry_appended_to_the_number() {
    check(&[
        ("de", "3.14", "dɾˈaɪ pʊŋkt fˈɪɾtseːn"),
        ("de", "1.2.3", "ˈaɪns pʊŋkt tsvˈaɪ pʊŋkt dɾˈaɪ"),
        ("es", "3.14", "tɾˈes punto katˈoɾθe"),
        ("hu", "3.14", "hˈaːrom pont tˈizɛnneːɟ"),
        ("fr", "3.14", "tʁwa pwɛ̃ katˈɔʁz"),
    ]);
}

#[test]
fn language_without_a_point_entry_reads_silence() {
    check(&[
        ("nl", "3.14", "drˈi fˈɪːrtin"),
        ("nl", "8.30", " ˈɑx tˈɛrtəx"),
        ("pt", "3.14", "tɹˈeʃ kɐtˈorzɨ"),
        ("pl", "3.14", "tʃˈɨ tʃtɛrnˈaɕtɕɛ"),
        ("nl", "1.2.3", " ˈeːn tʋˈeː drˈi"),
    ]);
}

#[test]
fn continued_decimal_point_fuses_to_the_previous_number() {
    check(&[
        ("en", "1.2.3", "wˈɒn pɔɪnt tˈuːpɔɪnt θɹˈiː"),
        ("en", "2.0.1", "tˈuː pɔɪnt zˈiəɹəʊpɔɪnt wˈɒn"),
        ("en", "10.25.2024", "tˈɛn pɔɪnt tˈuː fˈaɪvpɔɪnt tˈuː zˈiəɹəʊ tˈuː fˈɔː"),
    ]);
}
