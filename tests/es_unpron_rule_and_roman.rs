// A `$unpron` rule only serves the unpronounceable test: when the word is
// translated, the rule's phonemes are used and the word goes on (`xilófono`).
// Spanish also reads Roman numerals, as a number.
//
// Values from C espeak-ng master (f0fb11a8).

use espeak_ng::Translator;

fn es() -> Option<Translator> {
    let t = Translator::new("es", Some(std::path::Path::new("espeak-ng-data"))).ok()?;
    t.text_to_ipa("a").ok()?;
    Some(t)
}

#[test]
fn word_initial_x_continues_the_word() {
    let Some(t) = es() else { return };
    for (word, ipa) in [("xilófono", "silˈofono"), ("xanadu", "sanˈaðu"), ("xiao", "sjˈao")] {
        assert_eq!(t.text_to_ipa(word).unwrap(), ipa, "{word}");
    }
}

#[test]
fn roman_numerals_read_as_numbers() {
    let Some(t) = es() else { return };
    for (text, ipa) in [
        ("xi", "ˈonθe"),
        ("siglo XX", "sˈiɣlo βˈeɪnte"),
        ("Juan XXIII", "xwˈan bˌeɪntitɾˈes"),
    ] {
        assert_eq!(t.text_to_ipa(text).unwrap(), ipa, "{text}");
    }
}
